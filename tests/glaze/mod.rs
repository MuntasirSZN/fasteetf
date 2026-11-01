// Glaze-derived ETF tests from eetf_test.cpp
// Not all glaze tests apply to fasteetf — only patterns that match
// our Term enum and encoding/decoding architecture.

mod eetf_to_json_edge_cases;
mod map_proplist;
mod roundtrip_glaze;
mod truncated_buffer_guards;
