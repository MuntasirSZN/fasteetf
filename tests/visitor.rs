// ─────────────────────────────────────────────────────────────────────────────
// Integration tests for the Visitor API.
// ─────────────────────────────────────────────────────────────────────────────

#![cfg(feature = "alloc")]

use fasteetf::*;

#[test]
fn test_visitor_small_int() {
    use fasteetf::Visitor;
    struct IntCatcher(Option<i32>);
    impl Visitor for IntCatcher {
        type Error = EtfError;
        fn visit_int(&mut self, value: i32) -> Result<(), Self::Error> {
            self.0 = Some(value);
            Ok(())
        }
    }

    let mut v = IntCatcher(None);
    parse_etf_with_visitor(b"\x83\x61\x2a", None, &mut v, &Limits::default()).unwrap();
    assert_eq!(v.0, Some(42));
}

#[test]
fn test_visitor_tuple() {
    use fasteetf::Visitor;
    struct TupleCatcher {
        starts: usize,
        ends: usize,
    }
    impl Visitor for TupleCatcher {
        type Error = EtfError;
        fn visit_tuple_start(&mut self, _arity: usize) -> Result<(), Self::Error> {
            self.starts += 1;
            Ok(())
        }
        fn visit_tuple_end(&mut self) -> Result<(), Self::Error> {
            self.ends += 1;
            Ok(())
        }
    }

    let mut v = TupleCatcher { starts: 0, ends: 0 };
    parse_etf_with_visitor(
        b"\x83\x68\x02\x61\x01\x61\x02",
        None,
        &mut v,
        &Limits::default(),
    )
    .unwrap();
    assert_eq!(v.starts, 1);
    assert_eq!(v.ends, 1);
}

#[test]
fn test_visitor_integer_ext() {
    use fasteetf::Visitor;
    struct IntCatcher(Option<i32>);
    impl Visitor for IntCatcher {
        type Error = EtfError;
        fn visit_int(&mut self, value: i32) -> Result<(), Self::Error> {
            self.0 = Some(value);
            Ok(())
        }
    }

    // INTEGER_EXT: 255
    let mut v = IntCatcher(None);
    parse_etf_with_visitor(
        b"\x83\x62\x00\x00\x00\xff",
        None,
        &mut v,
        &Limits::default(),
    )
    .unwrap();
    assert_eq!(v.0, Some(255));

    // INTEGER_EXT: -1
    let mut v = IntCatcher(None);
    parse_etf_with_visitor(
        b"\x83\x62\xff\xff\xff\xff",
        None,
        &mut v,
        &Limits::default(),
    )
    .unwrap();
    assert_eq!(v.0, Some(-1));
}

#[test]
fn test_visitor_nil_list_balanced() {
    use fasteetf::Visitor;
    struct ListTracker {
        starts: usize,
        ends: usize,
    }
    impl Visitor for ListTracker {
        type Error = EtfError;
        fn visit_list_start(&mut self, _len: usize) -> Result<(), Self::Error> {
            self.starts += 1;
            Ok(())
        }
        fn visit_list_end(&mut self) -> Result<(), Self::Error> {
            self.ends += 1;
            Ok(())
        }
    }

    // NIL_EXT should produce balanced start/end.
    let mut v = ListTracker { starts: 0, ends: 0 };
    parse_etf_with_visitor(b"\x83\x6a", None, &mut v, &Limits::default()).unwrap();
    assert_eq!(v.starts, 1);
    assert_eq!(v.ends, 1);
}

#[test]
fn test_visitor_proper_list_balanced() {
    use fasteetf::Visitor;
    struct ListTracker {
        starts: usize,
        ends: usize,
    }
    impl Visitor for ListTracker {
        type Error = EtfError;
        fn visit_list_start(&mut self, _len: usize) -> Result<(), Self::Error> {
            self.starts += 1;
            Ok(())
        }
        fn visit_list_end(&mut self) -> Result<(), Self::Error> {
            self.ends += 1;
            Ok(())
        }
    }

    // [1, 2] -> LIST_EXT len=2, SMALL_INT 1, SMALL_INT 2, NIL_EXT
    let mut v = ListTracker { starts: 0, ends: 0 };
    parse_etf_with_visitor(
        b"\x83\x6c\x00\x00\x00\x02\x61\x01\x61\x02\x6a",
        None,
        &mut v,
        &Limits::default(),
    )
    .unwrap();
    assert_eq!(v.starts, 1);
    assert_eq!(v.ends, 1);
}

#[test]
fn test_visitor_nested() {
    use fasteetf::Visitor;
    struct DepthTracker(usize, usize); // max_depth, current_depth
    impl Visitor for DepthTracker {
        type Error = EtfError;
        fn visit_tuple_start(&mut self, _arity: usize) -> Result<(), Self::Error> {
            self.1 += 1;
            self.0 = self.0.max(self.1);
            Ok(())
        }
        fn visit_tuple_end(&mut self) -> Result<(), Self::Error> {
            self.1 -= 1;
            Ok(())
        }
    }

    let mut v = DepthTracker(0, 0);
    // tuple(tuple(1))
    let input = b"\x83\x68\x01\x68\x01\x61\x01";
    parse_etf_with_visitor(input, None, &mut v, &Limits::default()).unwrap();
    assert_eq!(v.0, 2);
}
