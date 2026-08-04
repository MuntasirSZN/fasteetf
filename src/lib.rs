//! A fast, `no_std` EETF (Erlang External Term Format) parser, encoder, and
//! (de)serialiser, as used by the Erlang VM, Elixir, and the BEAM ecosystem.
//!
//! Supports the full ETFP v1.1 spec including atom cache, bitstrings, maps,
//! large atom refs, and compressed payloads. The core parser and encoder
//! are zero-copy and require no allocator.
//!
//! # Feature flags
//!
//! | Feature              | What it adds                                      |
//! |----------------------|---------------------------------------------------|
//! | `std` *(default)*    | `std::error::Error` impls (implies `alloc`).       |
//! | `alloc`              | Owned types (`OwnedTerm`, …) + `encode_to_vec`.    |
//! | `compression` *(default)* | `COMPRESSED`-tag decode + `encode_to_compressed`. |
//! | `serde`              | `Serialize`/`Deserialize` for [`Term`].            |
//! | `zlib-rs` *(default)*     | Pure-Rust zlib backend.                        |
//! | `miniz_oxide`        | Alternate pure-Rust zlib backend.                  |
//! | `zlib` / `zlib-default` / `zlib-ng-compat` / `zlib-ng` / `cloudflare-zlib` | System/C lib backends. |
//!
//! # Example
//! ```rust
//! # use fasteetf::encode_to_buf;
//! let atom = fasteetf::Term::from("hello");
//! let mut buf = [0u8; 64];
//! let n = encode_to_buf(&atom, &mut buf).unwrap();
//! ```

#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

// ── Internal modules ────────────────────────────────────────────────────────

mod api;
mod arena;
mod cursor;
mod encoder;
mod error;
mod limits;
mod parser;
#[cfg(feature = "serde")]
mod serde;
mod simd;
mod tags;
mod types;
mod visitor;
mod zlib;

#[cfg(kani)]
mod proofs;

// ── Public API surface ──────────────────────────────────────────────────────

pub use api::{ParseOptions, parse_etf, parse_etf_streaming};
pub use encoder::encode_to_buf;

#[cfg(feature = "compression")]
pub use encoder::encode_to_compressed;
#[cfg(feature = "alloc")]
pub use encoder::encode_to_vec;
pub use error::{EtfError, Needed};
pub use limits::*;
pub use types::{AtomUtf8, Function, Pid, Port, Record, Reference, Term};
pub use visitor::{Visitor, parse_etf_with_visitor, parse_etf_with_visitor_streaming};
#[cfg(feature = "compression")]
pub use zlib::ZlibCompressFn;
pub use zlib::{ZlibBackend, ZlibDecompressFn};

#[cfg(feature = "alloc")]
pub use types::owned::{
    self, FunctionOwned, OwnedTerm, PidOwned, PortOwned, RecordOwned, ReferenceOwned,
};

// ── Constants ───────────────────────────────────────────────────────────────

/// Magic version byte.  Every valid ETF stream starts with `131`.
///
/// Spec: https://www.erlang.org/doc/apps/erts/erl_ext_dist#introduction
pub(crate) const ETF_MAGIC: u8 = 131;
