use fasteetf::Term;

use crate::common::with_parse;

#[test]
fn test_string_ext_decodes_like_erlpack_charlist() {
    with_parse(b"\x83k\x00\x0bhello world", |term| match term {
        Term::List(values) => {
            let actual: Vec<_> = values
                .iter()
                .map(|term| match term {
                    Term::Int(value) => *value,
                    other => panic!("expected integer in string list, got {other:?}"),
                })
                .collect();
            assert_eq!(
                actual,
                b"hello world"
                    .iter()
                    .map(|&value| value as i32)
                    .collect::<Vec<_>>()
            );
        }
        other => panic!("expected list, got {other:?}"),
    });
}

#[test]
fn test_charlist_decodes_large_integer_elements() {
    with_parse(
        b"\x83l\x00\x00\x00\x04b\x00\x00\x04\x00b\x00\x00\x04\x01b\x00\x00\x04\x02b\x00\x00\x04\x03j",
        |term| match term {
            Term::List(values) => {
                let actual: Vec<_> = values
                    .iter()
                    .map(|term| match term {
                        Term::Int(value) => *value,
                        other => panic!("expected integer, got {other:?}"),
                    })
                    .collect();
                assert_eq!(actual, [1024, 1025, 1026, 1027]);
            }
            other => panic!("expected list, got {other:?}"),
        },
    );
}

#[test]
fn test_utf8_atom_decodes_like_erlpack() {
    with_parse(
        b"\x83w\x15\xe3\x81\x93\xe3\x82\x93\xe3\x81\xab\xe3\x81\xa1\xe3\x81\xaf\xe4\xb8\x96\xe7\x95\x8c",
        |term| match term {
            Term::Atom(atom) => assert_eq!(atom.as_str().unwrap(), "こんにちは世界"),
            other => panic!("expected atom, got {other:?}"),
        },
    );
}
