// ─────────────────────────────────────────────────────────────────────────────
// Benchmark: ETF parse/encode/visitor throughput
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
//   6. Atoms         — 1000 small atoms
//   7. Large tuple   — 1000-element tuple (arena allocation)
//   8. STRING_EXT    — expanded (List of Ints) vs compact (Term::String)
//   9. Encode        — encode_to_buf vs encode_to_vec
//  10. Visitor       — zero-allocation visitor vs arena parsing
//
// Run with:
//   cargo bench
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(not(target_arch = "wasm32"))]
mod main {
    use fasteetf::{
        AtomUtf8, EtfError, Limits, ParseOptions, Term, Visitor, encode_to_buf, encode_to_vec,
        parse_etf, parse_etf_with_visitor,
    };
    use std::sync::LazyLock;

    #[global_allocator]
    static ALLOC: divan::AllocProfiler = divan::AllocProfiler::system();

    pub fn main() {
        divan::main();
    }

    // ── Helpers ─────────────────────────────────────────────────────────────────

    /// Capacity (and allocation limit) of the AST arena.
    const ARENA_CAPACITY: usize = 65536;

    /// Build a pre-allocated AST arena.  Benches create it in divan's untimed
    /// input setup, so only parsing is measured (the base implementation used
    /// an uninitialised stack buffer with no allocation cost).
    pub fn new_arena() -> bumpalo::Bump<16> {
        let arena = bumpalo::Bump::<16>::with_min_align_and_capacity(ARENA_CAPACITY);
        arena.set_allocation_limit(Some(ARENA_CAPACITY));
        arena
    }

    /// Parse ETF bytes into the given pre-allocated arena.
    pub fn parse_bytes(arena: &bumpalo::Bump<16>, input: &[u8]) {
        parse_bytes_with_limits(arena, input, Limits::default());
    }

    /// Parse with custom limits into the given pre-allocated arena.
    pub fn parse_bytes_with_limits(arena: &bumpalo::Bump<16>, input: &[u8], limits: Limits) {
        parse_etf(ParseOptions {
            input,
            decompressed_buffer: None,
            ast_arena: arena,
            limits,
            zlib_backend: None,
        })
        .unwrap();
    }

    /// A no-op visitor (all trait methods have default `Ok` implementations).
    struct NoopVisitor;

    impl Visitor for NoopVisitor {
        type Error = EtfError;
    }

    /// Build the wire bytes for a 1000-element flat list of small ints.
    pub fn flat_list_1000_wire() -> Vec<u8> {
        let mut input = Vec::with_capacity(6 + 2000);
        input.push(131);
        input.push(108); // LIST_EXT
        input.extend_from_slice(&(1000u32).to_be_bytes());
        for _ in 0..1000 {
            input.push(97); // SMALL_INTEGER_EXT
            input.push(1);
        }
        input.push(106); // NIL_EXT
        input
    }

    // ── Wire fixtures (built once) ──────────────────────────────────────────────

    static WIRE_MIXED_MAP_10: LazyLock<Vec<u8>> = LazyLock::new(|| {
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
        input
    });
    static WIRE_STRING_EXT: LazyLock<Vec<u8>> = LazyLock::new(|| {
        let mut input = Vec::with_capacity(6 + 512);
        input.push(131);
        input.push(107); // STRING_EXT
        input.extend_from_slice(&(512u16).to_be_bytes());
        input.extend(std::iter::repeat_n(b'a', 512));
        input
    });
    static WIRE_ATOMS_1000: LazyLock<Vec<u8>> = LazyLock::new(|| {
        let mut input = Vec::with_capacity(3 + 1000 * 7);
        input.push(131);
        for _ in 0..1000 {
            input.push(119); // SMALL_ATOM_UTF8_EXT
            input.push(5);
            input.extend_from_slice(b"hello");
        }
        input
    });

    // ── Encode fixtures (hand-built const terms — no heap, no leaks) ────────────

    /// 1000-element flat list of small ints, mirroring `WIRE_FLAT_LIST_1000`.
    static LIST_ELEMS_1000: [Term<'static>; 1000] = [Term::Int(1); 1000];
    static TERM_FLAT_LIST_1000: Term<'static> = Term::List(&LIST_ELEMS_1000);

    /// 10-pair map of atom keys to small-int values, mirroring `WIRE_MIXED_MAP_10`.
    static MAP_PAIRS_10: [(Term<'static>, Term<'static>); 10] =
        [(Term::Atom(AtomUtf8(b"key0")), Term::Int(0)); 10];
    static TERM_MIXED_MAP_10: Term<'static> = Term::Map(&MAP_PAIRS_10);

    // ── Benches ─────────────────────────────────────────────────────────────────

    #[divan::bench]
    pub fn small_int(bencher: divan::Bencher) {
        bencher.with_inputs(new_arena).bench_local_refs(|arena| {
            parse_bytes(arena, b"\x83\x61\x2a");
        });
    }

    #[divan::bench]
    pub fn flat_list_1000(bencher: divan::Bencher) {
        bencher.with_inputs(new_arena).bench_local_refs(|arena| {
            parse_bytes(arena, &flat_list_1000_wire());
        });
    }

    #[divan::bench]
    pub fn deep_nested_64(bencher: divan::Bencher) {
        bencher.with_inputs(new_arena).bench_local_refs(|arena| {
            let mut input = Vec::new();
            input.push(131);
            for _ in 0..64 {
                input.push(108); // LIST_EXT
                input.extend_from_slice(&1u32.to_be_bytes());
            }
            input.push(97); // SMALL_INTEGER_EXT
            input.push(42);
            input.extend(std::iter::repeat_n(106, 64)); // NIL_EXT
            parse_bytes(arena, &input);
        });
    }

    #[divan::bench]
    pub fn binary_1mb(bencher: divan::Bencher) {
        bencher.with_inputs(new_arena).bench_local_refs(|arena| {
            let mut input = Vec::with_capacity(6 + 1024 * 1024);
            input.push(131);
            input.push(109); // BINARY_EXT
            input.extend_from_slice(&(1024u32 * 1024).to_be_bytes());
            input.resize(input.capacity(), 0xAB);
            parse_bytes(arena, &input);
        });
    }

    #[divan::bench]
    pub fn mixed_map_10(bencher: divan::Bencher) {
        bencher.with_inputs(new_arena).bench_local_refs(|arena| {
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
            parse_bytes(arena, &input);
        });
    }

    // ── Deferred benchmark infrastructure (TODO.md) ─────────────────────────────

    /// Many small atoms (SMALL_ATOM_UTF8_EXT) — measures the atom fast paths.
    #[divan::bench]
    pub fn atom_parsing(bencher: divan::Bencher) {
        bencher.with_inputs(new_arena).bench_local_refs(|arena| {
            parse_bytes(arena, &WIRE_ATOMS_1000);
        });
    }

    /// A large tuple (LARGE_TUPLE_EXT, 1000 elements) — measures arena allocation.
    #[divan::bench]
    pub fn tuple_parsing(bencher: divan::Bencher) {
        bencher.with_inputs(new_arena).bench_local_refs(|arena| {
            let mut input = Vec::with_capacity(6 + 2000);
            input.push(131);
            input.push(105); // LARGE_TUPLE_EXT
            input.extend_from_slice(&(1000u32).to_be_bytes());
            for _ in 0..1000 {
                input.push(97); // SMALL_INTEGER_EXT
                input.push(1);
            }
            parse_bytes(arena, &input);
        });
    }

    /// STRING_EXT with the legacy expansion (List of Ints) — default limits.
    #[divan::bench]
    pub fn string_ext_expand(bencher: divan::Bencher) {
        bencher.with_inputs(new_arena).bench_local_refs(|arena| {
            parse_bytes(arena, &WIRE_STRING_EXT);
        });
    }

    /// STRING_EXT with the compact `Term::String` representation.
    #[divan::bench]
    pub fn string_ext_compact(bencher: divan::Bencher) {
        bencher.with_inputs(new_arena).bench_local_refs(|arena| {
            parse_bytes_with_limits(
                arena,
                &WIRE_STRING_EXT,
                Limits {
                    expand_string_ext_to_list: false,
                    ..Limits::default()
                },
            );
        });
    }

    /// Encode a 1000-element list into a fixed caller buffer.
    #[divan::bench]
    pub fn encode_to_buf_list_1000() {
        let mut buf = [0u8; 65536];
        encode_to_buf(&TERM_FLAT_LIST_1000, &mut buf).unwrap();
    }

    /// Encode a 1000-element list into a growable Vec.
    #[divan::bench]
    pub fn encode_to_vec_list_1000() {
        encode_to_vec(&TERM_FLAT_LIST_1000).unwrap();
    }

    /// Encode a 10-pair map into a fixed caller buffer.
    #[divan::bench]
    pub fn encode_to_buf_mixed_map_10() {
        let mut buf = [0u8; 65536];
        encode_to_buf(&TERM_MIXED_MAP_10, &mut buf).unwrap();
    }

    /// Encode a 10-pair map into a growable Vec.
    #[divan::bench]
    pub fn encode_to_vec_mixed_map_10() {
        encode_to_vec(&TERM_MIXED_MAP_10).unwrap();
    }

    /// AST (arena) parsing of the mixed map.
    #[divan::bench]
    pub fn visitor_vs_ast_parse(bencher: divan::Bencher) {
        bencher.with_inputs(new_arena).bench_local_refs(|arena| {
            parse_bytes(arena, &WIRE_MIXED_MAP_10);
        });
    }

    /// Zero-allocation visitor parsing of the mixed map.
    #[divan::bench]
    pub fn visitor_vs_ast_visit() {
        let mut visitor = NoopVisitor;
        parse_etf_with_visitor(
            &WIRE_MIXED_MAP_10,
            None,
            None,
            &mut visitor,
            &Limits::default(),
        )
        .unwrap();
    }
}

#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    main::main();
}
