<a name="v0.2.0"></a>

## [v0.2.0](https://github.com/MuntasirSZN/getquotes/compare/v0.1.0...v0.2.0) (2026-08-04)

### ✨ Features

- `Term` and `OwnedTerm` now implement `PartialEq`, `Eq`, and `Hash`; `ParseOptions` gets a manual `Debug` impl
- Shrink `Term` from 32 to 24 bytes (breaking): merge `BigInt` into a single variant, make `Pid`/`Port`/`Ref`/`Function` `&[u8]` aliases, and collapse `ImproperList` into one slice
- Add `Term::String` for memory-efficient `STRING_EXT` decoding (up to 32× less arena usage), configurable via `Limits::expand_string_ext_to_list`
- Add `Limits::max_bignum_size` for bignum payloads
- SIMD-accelerated atom comparison and copying: x86_64 (SSE2–AVX-512 with runtime detection), AArch64 NEON, wasm32 SIMD128
- Lower MSRV from 1.95 to 1.89
- Add Kani proofs and Miri, ASan, and fuzz CI workflows


### 🐞 Bug Fixes

- **zlib:** size the `out_len` slot for both zlib ABIs so decode works on 32-bit Windows
- Fix `simd_eq` comparing only the first block for lengths that aren't block multiples, which made prefix-sharing atoms compare equal
- Fix `just check`/`just test` failures in serde opaque wrappers and feature-matrix compression runs [#2](https://github.com/MuntasirSZN/getquotes/pull/2)
- Fix fuzzer findings in the encoder and the opaque parser
- Fix Kani proof failures
- Fix a zlib usage bug and the codecov CI job name
- Fix macOS clippy, typos, the wasm build, and the fuzz target


### ⌛ Performance Improvements

- Replace the recursive parser with an iterative frame-based one (1000-int flat list: ~11.5 µs → 3.5 µs)
- SIMD atom comparison and copying in `simd_eq`/`simd_copy`
- Faster arena, cursor, and visitor fast paths


### 🧰 Miscellaneous

- Modularize the source tree: parser, visitor, encoder, zlib backends, serde, and types as directory modules
- Rewrite the README and trim crate-level docs


<a name="v0.1.0"></a>

## v0.1.0 (2026-08-01)

### ✨ Features

- baseline
- feat: add tests (95% coverage), fuzzing, more zlib backends, compressed


### 🐞 Bug Fixes

- ulong
- doctest


