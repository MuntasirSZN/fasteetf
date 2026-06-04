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
    // Stack-allocated arena large enough for most well-formed terms.
    let mut arena_buf = [core::mem::MaybeUninit::<u8>::uninit(); 65536];

    // ── Step 1: Parse ─────────────────────────────────
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
    let mut arena_buf2 = [core::mem::MaybeUninit::<u8>::uninit(); 65536];
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
