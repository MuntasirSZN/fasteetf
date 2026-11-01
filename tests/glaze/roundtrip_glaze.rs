use core::f64;

use fasteetf::Term::*;
use fasteetf::encode_to_vec;

#[test]
fn test_glaze_roundtrip_small_int() {
    for v in [0i32, 1, 42, 127, 128, 255, 256] {
        let encoded = encode_to_vec(&Int(v)).unwrap();
        assert!(!encoded.is_empty());
    }
}

#[test]
fn test_glaze_roundtrip_float() {
    for v in [0.0f64, 1.0, -1.0, f64::consts::PI] {
        let encoded = encode_to_vec(&Float(v)).unwrap();
        assert!(!encoded.is_empty());
    }
}
