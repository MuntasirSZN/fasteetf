// ─────────────────────────────────────────────────────────────────────────────
// Fuzz target: raw byte-level ETF parsing.
//
// This target feeds arbitrary byte sequences into the parser and checks
// that it never panics or enters an infinite loop.
//
// Critical attack surfaces exercised:
//
//   * Truncated input  (every length field can be hostile)
//   * Invalid tags     (unknown discriminants)
//   * Excessive depths (recursion-bomb)
//   * Large length fields (allocation-bomb via arena exhaustion)
//   * Malformed UTF-8  (atoms)
//   * Integer overflow  (size arithmetic)
//
// Run with:
//   cargo +nightly fuzz run parse --features alloc
// ─────────────────────────────────────────────────────────────────────────────

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // We use a stack-allocated arena that is large enough for most
    // well-formed terms but will gracefully return ArenaExhausted for
    // pathological inputs.
    let mut arena_buf = [core::mem::MaybeUninit::<u8>::uninit(); 8192];

    let opts = fasteetf::ParseOptions {
        input: data,
        decompressed_buffer: None,
        ast_arena: &mut arena_buf,
        limits: fasteetf::Limits::default(),
    };

    // The parser MUST NEVER panic on any input.
    let _ = fasteetf::parse_etf(opts);
});
