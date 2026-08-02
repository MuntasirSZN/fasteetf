// ─────────────────────────────────────────────────────────────────────────────
// Serde serialization / deserialization integration tests.
// ─────────────────────────────────────────────────────────────────────────────

#![cfg(feature = "serde")]

#[path = "../common/mod.rs"]
mod common;
use common::*;
use fasteetf::*;

mod deserialize;
mod opaque;
mod serialize;
mod visitor;
