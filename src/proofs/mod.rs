// ─────────────────────────────────────────────────────────────────────────────
// Kani verification harnesses.
//
// Only compiled under `cargo kani` (`cfg(kani)`).  Each submodule targets one
// area of the codebase; every harness is small and loop-free (or bounded by
// constants), so the full suite verifies in seconds rather than minutes.
// ─────────────────────────────────────────────────────────────────────────────

mod arena;
mod cursor;
mod encoder;
mod error;
mod limits;
mod tags;
