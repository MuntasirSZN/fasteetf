# fasteetf

A extremely fast, Rust-based, `no_std` Erlang External Term Format (ETF) parser/encoder/(de)serializer.

[![Crates.io](https://img.shields.io/crates/v/fasteetf.svg)](https://crates.io/crates/fasteetf)
[![Documentation](https://docs.rs/fasteetf/badge.svg)](https://docs.rs/fasteetf)
[![License: LGPL-3.0-or-later](https://img.shields.io/badge/License-LGPL--3.0--or--later-blue.svg)](https://www.gnu.org/licenses/lgpl-3.0.en.html)
[![CI](https://github.com/MuntasirSZN/fasteetf/actions/workflows/ci.yml/badge.svg)](https://github.com/MuntasirSZN/fasteetf/actions/workflows/ci.yml)

## Overview

`fasteetf` provides a high-performance, zero-copy parser and encoder for the Erlang External Term Format (ETF), the binary format used by Erlang/Elixir for inter-process communication. It is designed to be:

- **Fast**: Optimized for throughput with minimal allocations
- **`no_std` compatible**: Runs on bare-metal, embedded systems, and WASM
- **Flexible**: Choose your features - from minimal parsing to full serialization with compression
- **Safe**: Written in pure Rust with no undefined behavior (verified with Miri)

## Features

`fasteetf` uses a modular feature system. The core parser works without `std` or `alloc`, and you can enable additional functionality as needed:

### Feature Matrix

| Feature | What it adds | Pulls in |
|---------|-------------|----------|
| `std` | `std::error::Error` impls on errors | `thiserror/std`, alloc |
| `alloc` | Owned types (`OwnedTerm`, …) and `encode_to_vec` | — |
| `compression` | COMPRESSED-tag decode + `encode_to_compressed` | — |
| `serde` | `Serialize`/`Deserialize` for `Term` and `OwnedTerm` | `serde_core` + alloc |
| `zlib-rs` | Built-in zlib backend (pure-Rust, `zlib-rs`) | `zlib-rs` |
| `miniz_oxide` | Built-in zlib backend (pure-Rust, `miniz_oxide`) | `miniz_oxide` |
| `zlib` | System zlib via `libz-sys` | `libz-sys` |
| `zlib-default` | System zlib via `libz-sys` with defaults | `libz-sys` |
| `zlib-ng-compat` | zlib-ng in compat mode via `libz-sys` | `libz-sys` |
| `zlib-ng` | Native zlib-ng via `libz-ng-sys` | `libz-ng-sys` |
| `cloudflare-zlib` | Cloudflare's zlib via `cloudflare-zlib-sys` | `cloudflare-zlib-sys` |

### Default Features

By default, `fasteetf` enables `std`, `compression`, and `zlib-rs`:

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

**Bare-metal / kernel / WASM, no compression, no alloc:**
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
use fasteetf::{parse_etf, Term, Limits};

// ETF-encoded small integer (42)
let data = [131, 97, 42]; // 131 = magic, 97 = SMALL_INTEGER_EXT, 42 = value

// Parse with a stack-allocated arena (no heap allocation)
let mut arena = [core::mem::MaybeUninit::uninit(); 1024];
let term = parse_etf(&data, &mut arena, Limits::default()).unwrap();

assert_eq!(term, Term::Int(42));
```

### Encoding ETF

```rust
use fasteetf::{encode_to_slice, Term, Limits};

let term = Term::Int(42);
let mut buf = [0u8; 64];
let written = encode_to_slice(&term, &mut buf, Limits::default()).unwrap();

// `written` contains the ETF-encoded bytes
assert_eq!(&buf[..written], &[131, 97, 42]);
```

### Using Owned Types (with `alloc`)

```rust
use fasteetf::{parse_etf_owned, OwnedTerm, Limits};

let data = [131, 97, 42];
let owned: OwnedTerm = parse_etf_owned(&data, Limits::default()).unwrap();

// OwnedTerm owns its data - no lifetime constraints
assert_eq!(owned, OwnedTerm::Int(42));
```

### Compression Support (with `compression`)

```rust
use fasteetf::{encode_to_compressed, parse_etf, Term, Limits};

let term = Term::Int(42);
let mut buf = [0u8; 128];
let written = encode_to_compressed(&term, &mut buf, Limits::default()).unwrap();

// Parse compressed ETF
let mut decomp = [0u8; 64];
let mut arena = [core::mem::MaybeUninit::uninit(); 1024];
let parsed = parse_etf(&buf[..written], &mut arena, Limits::default()).unwrap();
```

### Serde Support (with `serde`)

```rust
use fasteetf::{OwnedTerm, Limits};
use serde_json;

// Convert ETF to JSON
let data = [131, 97, 42];
let term: OwnedTerm = fasteetf::parse_etf_owned(&data, Limits::default()).unwrap();
let json = serde_json::to_string(&term).unwrap();

// Convert JSON to ETF
let term: OwnedTerm = serde_json::from_str(r#"{"ok": true}"#).unwrap();
```

## Performance

`fasteetf` is designed for high throughput parsing and encoding. Run the benchmarks:

```bash
cargo bench
```

## Documentation

Full documentation is available at [docs.rs/fasteetf](https://docs.rs/fasteetf).

## Examples

See the `examples/` directory for more usage examples.

## License

This project is licensed under the LGPL-3.0-or-later license. See the [LICENSE](https://github.com/MuntasirSZN/fasteetf/blob/main/LICENSE) file for details.

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## Acknowledgments

This project is inspired by the need for a fast, safe, and `no_std`-compatible ETF implementation for Rust.
