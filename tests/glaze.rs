// Glaze-derived test module (issue #23 — start with glaze)
// Runs with cargo-nextest always (see justfile/test recipe)

#![cfg_attr(not(doc), doc(hidden))]

#[path = "glaze/mod.rs"]
mod glaze;
