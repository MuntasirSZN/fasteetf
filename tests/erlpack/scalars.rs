use fasteetf::Term;

use crate::common;

#[test]
fn test_small_integers_match_erlpack_wire_format() {
    for value in [0, 1, 127, 128, 255] {
        assert_eq!(
            common::encode_ok(&Term::Int(value)),
            [0x83, 0x61, value as u8]
        );
    }
}

#[test]
fn test_integer_boundaries_match_erlpack_wire_format() {
    assert_eq!(
        common::encode_ok(&Term::Int(1024)),
        [0x83, 0x62, 0x00, 0x00, 0x04, 0x00]
    );
    assert_eq!(
        common::encode_ok(&Term::Int(i32::MIN)),
        [0x83, 0x62, 0x80, 0x00, 0x00, 0x00]
    );
    assert_eq!(
        common::encode_ok(&Term::Int(i32::MAX)),
        [0x83, 0x62, 0x7f, 0xff, 0xff, 0xff]
    );
}

#[test]
fn test_big_integers_match_erlpack_wire_format() {
    assert_eq!(
        common::encode_ok(&Term::BigInt {
            sign: 0,
            digits: &[0x00, 0x80, 0x00, 0x00],
        }),
        [0x83, 0x6e, 0x04, 0x00, 0x00, 0x80, 0x00, 0x00]
    );
    assert_eq!(
        common::encode_ok(&Term::BigInt {
            sign: 1,
            digits: &[0x00, 0x00, 0x00, 0x80],
        }),
        [0x83, 0x6e, 0x04, 0x01, 0x00, 0x00, 0x00, 0x80]
    );
}

#[test]
fn test_float_matches_erlpack_wire_format() {
    assert_eq!(
        common::encode_ok(&Term::Float(2.5)),
        [0x83, 0x46, 0x40, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
    );
}

#[test]
fn test_atom_boundaries_match_erlpack_wire_format() {
    let small = unsafe { fasteetf::AtomUtf8::from_bytes_unchecked(b"hello world") };
    assert_eq!(
        common::encode_ok(&Term::Atom(small)),
        [
            0x83, 0x77, 0x0b, b'h', b'e', b'l', b'l', b'o', b' ', b'w', b'o', b'r', b'l', b'd'
        ]
    );

    let bytes = vec![b'x'; 256];
    let large = unsafe { fasteetf::AtomUtf8::from_bytes_unchecked(&bytes) };
    let encoded = common::encode_ok(&Term::Atom(large));
    assert_eq!(&encoded[..4], &[0x83, 0x76, 0x01, 0x00]);
    assert_eq!(&encoded[4..], &bytes);
}

#[test]
fn test_binary_preserves_embedded_null_bytes() {
    let data = b"hello\0 world";
    assert_eq!(
        common::encode_ok(&Term::Binary(data)),
        [
            0x83, 0x6d, 0x00, 0x00, 0x00, 0x0c, b'h', b'e', b'l', b'l', b'o', 0, b' ', b'w', b'o',
            b'r', b'l', b'd'
        ]
    );
}

#[test]
fn test_boolean_and_nil_atoms_match_erlpack() {
    for (name, bytes) in [
        ("true", &[0x83, 0x77, 0x04, b't', b'r', b'u', b'e'][..]),
        (
            "false",
            &[0x83, 0x77, 0x05, b'f', b'a', b'l', b's', b'e'][..],
        ),
        ("nil", &[0x83, 0x77, 0x03, b'n', b'i', b'l'][..]),
    ] {
        let atom = unsafe { fasteetf::AtomUtf8::from_bytes_unchecked(name.as_bytes()) };
        assert_eq!(common::encode_ok(&Term::Atom(atom)), bytes);
    }
}
