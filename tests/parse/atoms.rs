use super::*;

// ── Atoms (lazy UTF-8) ──────────────────────────────────────────────────────

#[test]
fn test_atom_utf8() {
    with_parse(b"\x83\x76\x00\x05hello", |term| match term {
        Term::Atom(a) => {
            assert_eq!(a.as_str(), Ok("hello"));
            assert_eq!(a.as_bytes(), b"hello");
        }
        _ => panic!("expected Atom"),
    });
}

#[test]
fn test_small_atom_utf8() {
    with_parse(b"\x83\x77\x03hi!", |term| match term {
        Term::Atom(a) => {
            assert_eq!(a.as_str(), Ok("hi!"));
        }
        _ => panic!("expected Atom"),
    });
}

#[test]
fn test_empty_atom() {
    with_parse(b"\x83\x77\x00", |term| match term {
        Term::Atom(a) => {
            assert!(a.is_empty());
            assert_eq!(a.len(), 0);
        }
        _ => panic!("expected Atom"),
    });
}

#[test]
fn test_atom_lazy_utf8() {
    with_parse(b"\x83\x77\x02\xff\xfe", |term| match term {
        Term::Atom(a) => {
            assert!(a.as_str().is_err());
            assert_eq!(a.as_bytes(), &[0xff, 0xfe]);
        }
        _ => panic!("expected Atom"),
    });
}

// ── AtomUtf8 ergonomics ─────────────────────────────────────────────────────

#[test]
fn test_atom_utf8_comparison() {
    with_parse(b"\x83\x76\x00\x04true", |term| match term {
        Term::Atom(a) => {
            assert_eq!(a, "true");
            assert_eq!("true", a);
        }
        _ => panic!("expected Atom"),
    });
}
