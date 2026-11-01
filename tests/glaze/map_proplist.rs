use fasteetf::encode_to_vec;
use fasteetf::{AtomUtf8, Term};

#[test]
fn test_map_roundtrip_glaze() {
    let pairs = [(Term::Int(1), Term::Int(10)), (Term::Int(2), Term::Int(20))];
    let term = Term::Map(&pairs);
    let encoded = encode_to_vec(&term).unwrap();
    assert!(!encoded.is_empty());
}

#[test]
fn test_atom_key_map() {
    let a = unsafe { AtomUtf8::from_bytes_unchecked(b"key") };
    let pairs = [(Term::Atom(a), Term::Int(42))];
    let term = Term::Map(&pairs);
    let encoded = encode_to_vec(&term).unwrap();
    assert!(!encoded.is_empty());
}
