use fasteetf::Term::{self, *};
use fasteetf::encode_to_vec;

#[test]
fn test_map_pattern() {
    let pairs = [(Int(1), Int(10))];
    let term = Term::Map(&pairs);
    let encoded = encode_to_vec(&term).unwrap();
    assert!(!encoded.is_empty());
}
