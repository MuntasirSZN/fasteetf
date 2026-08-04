// ─────────────────────────────────────────────────────────────────────────────
// Fuzz target: structure-aware ETF round-trip.
//
// For every byte sequence that parses successfully, the result is encoded
// back to ETF and re-parsed.  A crash in either encoding or the second
// parse indicates a bug in the round-trip (e.g. the encoder emits malformed
// output, or the parser chokes on its own output).
//
// Run with:
//   cargo +nightly fuzz run parse_structured --features alloc
// ─────────────────────────────────────────────────────────────────────────────

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Heap arena sized to the input: every input byte can expand into
    // several arena slots when re-encoded (strings expand to lists of
    // ints, tuples/lists allocate per-element slots plus a frame), so a
    // fixed stack buffer is too small for adversarial-but-valid inputs —
    // e.g. a 3.6 KiB input whose encoded form needs > 64 KiB of arena.
    // 512 B/byte covers the observed worst case with margin.
    fn arena_for(len: usize) -> Vec<core::mem::MaybeUninit<u8>> {
        let bytes = (len * 512).clamp(2 * 1024 * 1024, 64 * 1024 * 1024);
        vec![core::mem::MaybeUninit::uninit(); bytes]
    }

    // ── Step 1: Parse ─────────────────────────────────
    let mut arena_buf = arena_for(data.len());
    let opts = fasteetf::ParseOptions {
        input: data,
        decompressed_buffer: None,
        ast_arena: &mut arena_buf,
        limits: fasteetf::Limits::default(),
        zlib_backend: None,
    };

    let term = match fasteetf::parse_etf(opts) {
        Ok(t) => t,
        Err(_) => return, // malformed input — not a round-trip failure
    };

    // ── Step 2: Encode ─────────────────────────────────
    let encoded = match fasteetf::encode_to_vec(&term) {
        Ok(bytes) => bytes,
        Err(_) => panic!("encode_to_vec failed on a valid term"),
    };

    // ── Step 3: Re-parse ──────────────────────────────────
    let mut arena_buf2 = arena_for(encoded.len());
    let opts2 = fasteetf::ParseOptions {
        input: &encoded,
        decompressed_buffer: None,
        ast_arena: &mut arena_buf2,
        limits: fasteetf::Limits::default(),
        zlib_backend: None,
    };

    let term2 = match fasteetf::parse_etf(opts2) {
        Ok(t) => t,
        Err(e) => {
            panic!(
                "round-trip: parse → encode → re-parse failed: {e}\n\
                 original input bytes: {data:?}\n\
                 encoded bytes: {encoded:?}"
            );
        }
    };

    // ── Step 4: Compare structure (via debug representation) ─────────
    // A full structural comparison would require OwnedTerm + PartialEq,
    // but checking Debug output catches most semantic mismatches.
    let repr1 = format!("{term:?}");
    let repr2 = format!("{term2:?}");
    if repr1 != repr2 {
        panic!(
            "round-trip structural mismatch:\n\
             original: {repr1}\n\
             re-parsed: {repr2}\n\
             encoded bytes: {encoded:?}"
        );
    }
});
