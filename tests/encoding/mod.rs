// ─────────────────────────────────────────────────────────────────────────────
// Integration tests for ETF encoding and comprehensive encode→parse roundtrips.
// ─────────────────────────────────────────────────────────────────────────────

#![cfg(feature = "alloc")]

#[path = "../common/mod.rs"]
mod common;
use common::*;
use fasteetf::*;

mod basic;
mod buf;
mod roundtrip;
