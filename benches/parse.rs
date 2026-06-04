// ─────────────────────────────────────────────────────────────────────────────
// Benchmark: ETF parse throughput
//
// Uses divan — a simpler, faster benchmark harness than Criterion.
//
// Measures end-to-end decode throughput for a variety of term shapes:
//
//   1. Tiny scalar   — small integer
//   2. Flat list     — 1000-element list of small integers
//   3. Deeply nested — 64-deep nested lists
//   4. Binary blob   — 1 MiB binary
//   5. Mixed term    — map with atoms, ints
//
// Run with:
//   cargo bench
// ─────────────────────────────────────────────────────────────────────────────

use core::mem::MaybeUninit;
use fasteetf::{Limits, ParseOptions, parse_etf};

#[global_allocator]
static ALLOC: divan::AllocProfiler = divan::AllocProfiler::system();

fn main() {
    divan::main();
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Parse ETF bytes using a stack-allocated arena (no heap allocs).
fn parse_bytes(input: &[u8]) {
    let mut arena_buf = [MaybeUninit::<u8>::uninit(); 65536];
    parse_etf(ParseOptions {
        input,
        decompressed_buffer: None,
        ast_arena: &mut arena_buf,
        limits: Limits::default(),
        zlib_backend: None,
    })
    .unwrap();
}

// ── Benches ─────────────────────────────────────────────────────────────────

#[divan::bench]
fn small_int() {
    parse_bytes(b"\x83\x61\x2a");
}

#[divan::bench]
fn flat_list_1000() {
    let mut input = Vec::with_capacity(6 + 2000);
    input.push(131);
    input.push(108); // LIST_EXT
    input.extend_from_slice(&(1000u32).to_be_bytes());
    for _ in 0..1000 {
        input.push(97); // SMALL_INTEGER_EXT
        input.push(1);
    }
    input.push(106); // NIL_EXT
    parse_bytes(&input);
}

#[divan::bench]
fn deep_nested_64() {
    let mut input = Vec::new();
    input.push(131);
    for _ in 0..64 {
        input.push(108); // LIST_EXT
        input.extend_from_slice(&1u32.to_be_bytes());
    }
    input.push(97); // SMALL_INTEGER_EXT
    input.push(42);
    input.extend(std::iter::repeat_n(106, 64)); // NIL_EXT
    parse_bytes(&input);
}

#[divan::bench]
fn binary_1mb() {
    let mut input = Vec::with_capacity(6 + 1024 * 1024);
    input.push(131);
    input.push(109); // BINARY_EXT
    input.extend_from_slice(&(1024u32 * 1024).to_be_bytes());
    input.resize(input.capacity(), 0xAB);
    parse_bytes(&input);
}

#[divan::bench]
fn mixed_map_10() {
    let mut input = Vec::new();
    input.push(131);
    input.push(116); // MAP_EXT
    input.extend_from_slice(&10u32.to_be_bytes());
    for i in 0..10 {
        let key = format!("key{}", i);
        input.push(119); // SMALL_ATOM_UTF8_EXT
        input.push(key.len() as u8);
        input.extend_from_slice(key.as_bytes());
        input.push(97); // SMALL_INTEGER_EXT
        input.push(i);
    }
    parse_bytes(&input);
}
