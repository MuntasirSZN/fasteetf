// ─────────────────────────────────────────────────────────────────────────────
// Integration tests for ETF parsing — every tag variant with valid input,
// error cases, OwnedTerm conversion, atom ergonomics, and edge cases.
// ─────────────────────────────────────────────────────────────────────────────

#![cfg(feature = "alloc")]

#[path = "../common/mod.rs"]
mod common;
use common::*;
use fasteetf::*;

mod atoms;
mod compound;
mod edge;
mod errors;
mod opaque;
mod owned;
mod scalars;
