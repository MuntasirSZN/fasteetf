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

#[test]
fn test_atom_utf8_equality_and_ordering_api() {
    with_parse(b"\x83\x76\x00\x04true", |term| {
        let Term::Atom(a) = term else {
            panic!("expected Atom");
        };
        // AtomUtf8 == AtomUtf8 (SIMD path), Eq, and negation.
        assert_eq!(a, AtomUtf8::from("true"));
        assert_ne!(a, AtomUtf8::from("truE"));
        assert_ne!(a, AtomUtf8::from("tru"));
        // Comparison against a short string takes the scalar path.
        assert_eq!(a, "true");
        assert_ne!(a, "TRUE");
        // Hash: equal atoms hash alike.
        use core::hash::{Hash, Hasher};
        let hash_of = |atom: AtomUtf8<'_>| {
            let mut h = std::collections::hash_map::DefaultHasher::new();
            atom.hash(&mut h);
            h.finish()
        };
        assert_eq!(hash_of(a), hash_of(AtomUtf8::from("true")));
    });
}

#[test]
fn test_atom_utf8_long_atoms_simd_paths() {
    // 16-byte atoms hit the SSE2 comparison path, 32-byte atoms the AVX2
    // path, and 64-byte atoms the AVX-512 path on capable machines.
    let sixteen = b"abcdefghijklmnop";
    let thirty_two = b"abcdefghijklmnopqrstuvwxyz012345";
    let sixty_four = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ab";
    for (len, name) in [
        (16usize, &sixteen[..]),
        (32, &thirty_two[..]),
        (64, &sixty_four[..]),
    ] {
        let name = core::str::from_utf8(name).unwrap();
        let mut wire = vec![0x83, 0x76];
        wire.extend_from_slice(&(len as u16).to_be_bytes());
        wire.extend_from_slice(name.as_bytes());
        with_parse(&wire, |term| {
            let Term::Atom(a) = term else {
                panic!("expected Atom");
            };
            assert_eq!(a, name);
            assert_eq!(name, a);
            assert_eq!(a, AtomUtf8::from(name));
        });
    }
}

#[test]
fn test_atom_utf8_conversions() {
    // From<&str> for AtomUtf8, From<AtomUtf8> for Term, From<&str> for Term.
    let atom: AtomUtf8<'_> = AtomUtf8::from("hello");
    assert_eq!(atom.len(), 5);
    assert!(!atom.is_empty());
    assert_eq!(atom.as_str(), Ok("hello"));
    assert_eq!(atom.as_bytes(), b"hello");
    let term: Term<'_> = atom.into();
    assert!(matches!(term, Term::Atom(_)));
    let term: Term<'_> = "world".into();
    assert!(matches!(term, Term::Atom(_)));
    // Empty atom conversions.
    let empty: AtomUtf8<'_> = AtomUtf8::from("");
    assert!(empty.is_empty());
    assert_eq!(empty.len(), 0);
    // SAFETY: the byte string is valid UTF-8.
    let raw = unsafe { AtomUtf8::from_bytes_unchecked(b"raw") };
    assert_eq!(raw.as_bytes(), b"raw");
}
