// ─────────────────────────────────────────────────────────────────────────────
// Integration tests for streaming / incremental parsing and the Needed API.
// ─────────────────────────────────────────────────────────────────────────────

#![cfg(feature = "alloc")]

use fasteetf::*;

mod incremental;
mod truncation;
