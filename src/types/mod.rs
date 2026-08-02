// ─────────────────────────────────────────────────────────────────────────────
// Type definitions for decoded ETF terms.
//
// This module is split into:
// - `borrowed`: Zero-copy types that borrow from the input buffer
// - `owned`: Heap-allocated types that own their data (feature-gated behind `alloc`)
// ─────────────────────────────────────────────────────────────────────────────

// Re-export borrowed types from the borrowed submodule
pub use borrowed::*;

// Internal submodules
mod borrowed;
#[cfg(feature = "alloc")]
#[doc(hidden)]
pub mod owned;
