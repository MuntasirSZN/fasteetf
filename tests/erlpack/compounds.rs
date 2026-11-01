use fasteetf::Term;

use crate::common;

#[test]
fn test_list_matches_erlpack_wire_format() {
    let values = [
        Term::Int(1),
        Term::Binary(b"two"),
        Term::Float(3.0),
        Term::Binary(b"four"),
    ];
    let term = Term::List(&values);
    let encoded = common::encode_ok(&term);
    assert_eq!(
        encoded,
        [
            0x83, 0x6c, 0x00, 0x00, 0x00, 0x04, 0x61, 0x01, 0x6d, 0x00, 0x00, 0x00, 0x03, b't',
            b'w', b'o', 0x46, 0x40, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x6d, 0x00, 0x00,
            0x00, 0x04, b'f', b'o', b'u', b'r', 0x6a
        ]
    );
}

#[test]
fn test_empty_list_matches_erlpack_wire_format() {
    assert_eq!(common::encode_ok(&Term::List(&[])), [0x83, 0x6a]);
}

#[test]
fn test_small_and_large_tuple_tags_match_erlpack() {
    let small = [Term::Int(1), Term::Int(2), Term::Int(3)];
    assert_eq!(
        common::encode_ok(&Term::Tuple(&small)),
        [0x83, 0x68, 0x03, 0x61, 0x01, 0x61, 0x02, 0x61, 0x03]
    );

    let large: Vec<_> = (0..256).map(Term::Int).collect();
    let encoded = common::encode_ok(&Term::Tuple(&large));
    assert_eq!(&encoded[..6], &[0x83, 0x69, 0x00, 0x00, 0x01, 0x00]);
}

#[test]
fn test_map_matches_erlpack_wire_format() {
    let pairs = [
        (Term::Binary(b"a"), Term::Int(1)),
        (Term::Int(2), Term::Int(2)),
        (
            Term::Int(3),
            Term::List(&[Term::Int(1), Term::Int(2), Term::Int(3)]),
        ),
    ];
    assert_eq!(
        common::encode_ok(&Term::Map(&pairs)),
        [
            0x83, 0x74, 0x00, 0x00, 0x00, 0x03, 0x6d, 0x00, 0x00, 0x00, 0x01, b'a', 0x61, 0x01,
            0x61, 0x02, 0x61, 0x02, 0x61, 0x03, 0x6c, 0x00, 0x00, 0x00, 0x03, 0x61, 0x01, 0x61,
            0x02, 0x61, 0x03, 0x6a
        ]
    );
}
