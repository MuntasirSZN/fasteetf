# fasteetf

A extremely fast, Rust-based, [`no_std`](https://docs.rust-embedded.org/book/intro/no-std.html) [Erlang External Term Format (ETF)](https://www.erlang.org/doc/apps/erts/erl_ext_dist.html) parser/encoder/(de)serializer.

[![Crates.io](https://img.shields.io/crates/v/fasteetf.svg)](https://crates.io/crates/fasteetf)
[![Documentation](https://docs.rs/fasteetf/badge.svg)](https://docs.rs/fasteetf)
[![License: LGPL-3.0-or-later](https://img.shields.io/badge/License-LGPL--3.0--or--later-blue.svg)](https://www.gnu.org/licenses/lgpl-3.0.en.html)
[![CI](https://github.com/MuntasirSZN/fasteetf/actions/workflows/ci.yml/badge.svg)](https://github.com/MuntasirSZN/fasteetf/actions/workflows/ci.yml)

## Overview

`fasteetf` provides a high-performance, zero-copy parser and encoder for the [Erlang External Term Format (ETF)](https://www.erlang.org/doc/apps/erts/erl_ext_dist.html), the binary format used by Erlang/Elixir. It is designed to be:

- **Fast**: Optimized for throughput with minimal allocations, SIMD.
- **[`no_std`](https://docs.rust-embedded.org/book/intro/no-std.html) compatible**: Runs on bare-metal, embedded systems, and WASM
- **Flexible**: Choose your features - from minimal parsing to full serialization with compression
- **Safe**: Written in pure Rust with no undefined behavior (See [Safety](#safety) section)

## Features

`fasteetf` uses a modular feature system. The core parser works without [`std`](https://doc.rust-lang.org/std) or [`alloc`](https://doc.rust-lang.org/alloc/), and you can enable additional functionality as needed:

### Feature Matrix

| Feature | What it adds | Pulls in |
|---------|-------------|----------|
| [`std`](https://docs.rs/crate/fasteetf/latest/features#std) | [`std::error::Error`](https://doc.rust-lang.org/std/error/trait.Error.html) impls on errors | [`thiserror/std`](https://docs.rs/crate/thiserror/latest/features#std), [`alloc`](https://doc.rust-lang.org/alloc/) |
| [`alloc`](https://docs.rs/crate/fasteetf/latest/features#alloc) | Owned types ([`OwnedTerm`](https://docs.rs/fasteetf/latest/fasteetf/enum.OwnedTerm.html), …) and [`encode_to_vec`](https://docs.rs/fasteetf/latest/fasteetf/fn.encode_to_vec.html) | — |
| [`compression`](https://docs.rs/crate/fasteetf/latest/features#compression) | COMPRESSED-tag decode + [`encode_to_compressed`](https://docs.rs/fasteetf/latest/fasteetf/fn.encode_to_compressed.html) | — |
| [`serde`](https://docs.rs/crate/fasteetf/latest/features#serde) | [`Serialize`](https://docs.rs/serde/latest/serde/derive.Serialize.html)/[`Deserialize`](https://docs.rs/serde/latest/serde/derive.Deserialize.html) for [`Term`](https://docs.rs/fasteetf/latest/fasteetf/enum.Term.html) and [`OwnedTerm`](https://docs.rs/fasteetf/latest/fasteetf/enum.OwnedTerm.html) | [`serde_core`](https://docs.rs/serde_core) + [`alloc`](https://doc.rust-lang.org/alloc/) |
| [`zlib-rs`](https://docs.rs/crate/fasteetf/latest/features#zlib-rs) | Built-in zlib backend (pure-Rust, [`zlib-rs`](https://docs.rs/zlib-rs)) | [`zlib-rs`](https://docs.rs/zlib-rs) |
| [`miniz_oxide`](https://docs.rs/crate/fasteetf/latest/features#miniz_oxide) | Built-in zlib backend (pure-Rust, [`miniz_oxide`](https://docs.rs/miniz_oxide)) | [`miniz_oxide`](https://docs.rs/miniz_oxide) |
| [`zlib`](https://docs.rs/crate/fasteetf/latest/features#zlib) | System zlib via [`libz-sys`](https://docs.rs/libz-sys) | [`libz-sys`](https://docs.rs/libz-sys) |
| [`zlib-default`](https://docs.rs/crate/fasteetf/latest/features#zlib-default) | System zlib via [`libz-sys`](https://docs.rs/libz-sys) with defaults | [`libz-sys`](https://docs.rs/libz-sys) |
| [`zlib-ng-compat`](https://docs.rs/crate/fasteetf/latest/features#zlib-ng-compat) | zlib-ng in compat mode via [`libz-sys`](https://docs.rs/libz-sys) | [`libz-sys`](https://docs.rs/libz-sys) |
| [`zlib-ng`](https://docs.rs/crate/fasteetf/latest/features#zlib-ng) | Native zlib-ng via [`libz-ng-sys`](https://docs.rs/libz-ng-sys) | [`libz-ng-sys`](https://docs.rs/libz-ng-sys) |
| [`cloudflare-zlib`](https://docs.rs/crate/fasteetf/latest/features#cloudflare-zlib) | Cloudflare's zlib via [`cloudflare-zlib-sys`](https://docs.rs/cloudflare-zlib-sys) | [`cloudflare-zlib-sys`](https://docs.rs/cloudflare-zlib-sys) |

### Default Features

By default, `fasteetf` enables [`std`](https://docs.rs/crate/fasteetf/latest/features#std), [`compression`](https://docs.rs/crate/fasteetf/latest/features#compression), and [`zlib-rs`](https://docs.rs/crate/fasteetf/latest/features#zlib-rs):

```toml
fasteetf = "0.1"  # pulls in std, alloc, compression, zlib-rs
```

## Installation

Add `fasteetf` to your `Cargo.toml`:

```toml
[dependencies]
fasteetf = "0.1"
```

### Common Recipes

**Bare-metal / kernel / WASM, no compression, no [`alloc`](https://doc.rust-lang.org/alloc/):**
```toml
fasteetf = { version = "0.1", default-features = false }
```

**Embedded with std but no compression:**
```toml
fasteetf = { version = "0.1", default-features = false, features = ["std"] }
```

**Server-side with a different zlib backend:**
```toml
fasteetf = { version = "0.1", default-features = false, features = ["std", "compression", "miniz_oxide", "serde"] }
```

## Usage

### Parsing ETF

```rust
use fasteetf::{parse_etf, ParseOptions, Term, Limits};

// ETF-encoded small integer (42)
let data = [131, 97, 42]; // 131 = magic, 97 = SMALL_INTEGER_EXT, 42 = value

// Parse with a stack-allocated arena (no heap allocation)
let mut arena = [core::mem::MaybeUninit::uninit(); 1024];
let term = parse_etf(ParseOptions {
    input: &data,
    ast_arena: &mut arena,
    limits: Limits::default(),
    decompressed_buffer: None,
    zlib_backend: None,
})
.unwrap();

assert!(matches!(term, Term::Int(42)));
```

### Encoding ETF

```rust
use fasteetf::{encode_to_buf, Term};

let term = Term::Int(42);
let mut buf = [0u8; 64];
let written = encode_to_buf(&term, &mut buf).unwrap();

// `written` contains the ETF-encoded bytes
assert_eq!(&buf[..written], &[131, 97, 42]);
```

### Using Owned Types (with [`alloc`](https://docs.rs/crate/fasteetf/latest/features#alloc))

```rust
use fasteetf::{parse_etf, OwnedTerm, ParseOptions, Term, Limits};

let data = [131, 97, 42];
let mut arena = [core::mem::MaybeUninit::uninit(); 1024];
let term = parse_etf(ParseOptions {
    input: &data,
    ast_arena: &mut arena,
    limits: Limits::default(),
    decompressed_buffer: None,
    zlib_backend: None,
})
.unwrap();

// OwnedTerm owns its data - no lifetime constraints
let owned: OwnedTerm = OwnedTerm::from(term);
assert!(matches!(owned, OwnedTerm::Int(42)));
```

### Compression Support (with [`compression`](https://docs.rs/crate/fasteetf/latest/features#compression))

```rust
use fasteetf::{encode_to_compressed, parse_etf, ParseOptions, Term, Limits};

let term = Term::Int(42);
let mut intermediate = [0u8; 128];
let mut output = [0u8; 128];
let written = encode_to_compressed(&term, &mut intermediate, &mut output, None).unwrap();

// Parse compressed ETF
let mut decomp = [0u8; 64];
let mut arena = [core::mem::MaybeUninit::uninit(); 1024];
let parsed = parse_etf(ParseOptions {
    input: &output[..written],
    decompressed_buffer: Some(&mut decomp),
    zlib_backend: None,
    ast_arena: &mut arena,
    limits: Limits::default(),
})
.unwrap();

assert!(matches!(parsed, Term::Int(42)));
```

### Serde Support (with [`serde`](https://docs.rs/crate/fasteetf/latest/features#serde))

```rust
use fasteetf::{parse_etf, OwnedTerm, ParseOptions, Limits};
use serde_json;

// Convert ETF to JSON
let data = [131, 97, 42];
let mut arena = [core::mem::MaybeUninit::uninit(); 1024];
let term = parse_etf(ParseOptions {
    input: &data,
    ast_arena: &mut arena,
    limits: Limits::default(),
    decompressed_buffer: None,
    zlib_backend: None,
})
.unwrap();
let json = serde_json::to_string(&term).unwrap();
assert_eq!(json, "42");

// Convert JSON to an OwnedTerm
let term: OwnedTerm = serde_json::from_str(r#"{"ok": true}"#).unwrap();
assert!(matches!(term, OwnedTerm::Map(_)));
```

## Safety

This crate contains some unsafe code, specially SIMD and some unchecked operations. To ensure safety, the following measures are taken:

- The crate is thoroughly tested with unit tests and property-based tests via [`proptest`](https://crates.io/crates/proptest).
- The crate is verified with [Miri](https://github.com/rust-lang/miri) to catch undefined behavior.
- [AddressSanitizer](https://doc.rust-lang.org/beta/unstable-book/compiler-flags/sanitizer.html#addresssanitizer) is used in CI to detect memory errors.
- Formal verification of the parser is performed using [Kani](https://github.com/model-checking/kani) to ensure correctness and safety of the parsing logic.
- The crate is fuzzed with [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz) to find edge cases and potential vulnerabilities.

## Performance

`fasteetf` is designed for high throughput parsing and encoding. Run the benchmarks:

```bash
cargo bench
```

## Documentation

Full documentation is available at [docs.rs/fasteetf](https://docs.rs/fasteetf).

## License

This project is licensed under the LGPL-3.0-or-later license. See the [LICENSE](https://github.com/MuntasirSZN/fasteetf/blob/main/LICENSE) file for details.

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.
