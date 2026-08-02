// ── Visitor visit_* methods: drive each via a typed deserializer ───────────

#[test]
fn test_serde_ownedterm_visit_i8() {
    // Use serde_json with a small JSON value.  Numbers go through visit_i64
    // in serde_json; we instead create a custom deserializer that calls visit_i8.
    use fasteetf::owned::OwnedTerm;

    struct I8Deser(i8);
    impl<'de> serde_core::Deserializer<'de> for I8Deser {
        type Error = serde_json::Error;
        fn deserialize_any<V: serde_core::de::Visitor<'de>>(
            self,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            visitor.visit_i8(self.0)
        }
        serde_core::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }
    let val = <i8 as serde_core::Deserialize>::deserialize(I8Deser(42)).unwrap();
    assert_eq!(val, 42);

    // Now drive OwnedTerm through visit_i8: deserialize OwnedTerm via the
    // inner visitor, providing a deserializer that calls visit_i8.
    struct OwnedTermViaI8(i8);
    impl<'de> serde_core::Deserializer<'de> for OwnedTermViaI8 {
        type Error = serde_json::Error;
        fn deserialize_any<V: serde_core::de::Visitor<'de>>(
            self,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            visitor.visit_i8(self.0)
        }
        serde_core::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }
    let term: OwnedTerm = serde_core::Deserialize::deserialize(OwnedTermViaI8(7)).unwrap();
    assert!(matches!(term, OwnedTerm::Int(7)));
}

#[test]
fn test_serde_ownedterm_visit_i16() {
    use fasteetf::owned::OwnedTerm;
    struct I16Deser(i16);
    impl<'de> serde_core::Deserializer<'de> for I16Deser {
        type Error = serde_json::Error;
        fn deserialize_any<V: serde_core::de::Visitor<'de>>(
            self,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            visitor.visit_i16(self.0)
        }
        serde_core::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }
    let term: OwnedTerm = serde_core::Deserialize::deserialize(I16Deser(1000)).unwrap();
    assert!(matches!(term, OwnedTerm::Int(1000)));
}

#[test]
fn test_serde_ownedterm_visit_i32() {
    use fasteetf::owned::OwnedTerm;
    struct I32Deser(i32);
    impl<'de> serde_core::Deserializer<'de> for I32Deser {
        type Error = serde_json::Error;
        fn deserialize_any<V: serde_core::de::Visitor<'de>>(
            self,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            visitor.visit_i32(self.0)
        }
        serde_core::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }
    let term: OwnedTerm = serde_core::Deserialize::deserialize(I32Deser(123_456)).unwrap();
    assert!(matches!(term, OwnedTerm::Int(123_456)));
}

#[test]
fn test_serde_ownedterm_visit_u8() {
    use fasteetf::owned::OwnedTerm;
    struct U8Deser(u8);
    impl<'de> serde_core::Deserializer<'de> for U8Deser {
        type Error = serde_json::Error;
        fn deserialize_any<V: serde_core::de::Visitor<'de>>(
            self,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            visitor.visit_u8(self.0)
        }
        serde_core::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }
    let term: OwnedTerm = serde_core::Deserialize::deserialize(U8Deser(200)).unwrap();
    assert!(matches!(term, OwnedTerm::Int(200)));
}

#[test]
fn test_serde_ownedterm_visit_u16() {
    use fasteetf::owned::OwnedTerm;
    struct U16Deser(u16);
    impl<'de> serde_core::Deserializer<'de> for U16Deser {
        type Error = serde_json::Error;
        fn deserialize_any<V: serde_core::de::Visitor<'de>>(
            self,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            visitor.visit_u16(self.0)
        }
        serde_core::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }
    let term: OwnedTerm = serde_core::Deserialize::deserialize(U16Deser(40_000)).unwrap();
    assert!(matches!(term, OwnedTerm::Int(40_000)));
}

#[test]
fn test_serde_ownedterm_visit_u32() {
    use fasteetf::owned::OwnedTerm;
    struct U32Deser(u32);
    impl<'de> serde_core::Deserializer<'de> for U32Deser {
        type Error = serde_json::Error;
        fn deserialize_any<V: serde_core::de::Visitor<'de>>(
            self,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            visitor.visit_u32(self.0)
        }
        serde_core::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }
    // u32::MAX doesn't fit in i32 → SmallBigInt path
    let term: OwnedTerm = serde_core::Deserialize::deserialize(U32Deser(u32::MAX)).unwrap();
    assert!(matches!(term, OwnedTerm::SmallBigInt { sign: 0, .. }));
}

#[test]
fn test_serde_ownedterm_visit_f32() {
    use fasteetf::owned::OwnedTerm;
    struct F32Deser(f32);
    impl<'de> serde_core::Deserializer<'de> for F32Deser {
        type Error = serde_json::Error;
        fn deserialize_any<V: serde_core::de::Visitor<'de>>(
            self,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            visitor.visit_f32(self.0)
        }
        serde_core::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }
    let term: OwnedTerm = serde_core::Deserialize::deserialize(F32Deser(1.5)).unwrap();
    assert!(matches!(term, OwnedTerm::Float(_)));
}

#[test]
fn test_serde_ownedterm_visit_string() {
    use fasteetf::owned::OwnedTerm;
    struct StringDeser(String);
    impl<'de> serde_core::Deserializer<'de> for StringDeser {
        type Error = serde_json::Error;
        fn deserialize_any<V: serde_core::de::Visitor<'de>>(
            self,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            visitor.visit_string(self.0)
        }
        serde_core::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }
    let term: OwnedTerm =
        serde_core::Deserialize::deserialize(StringDeser("hello".to_string())).unwrap();
    assert!(matches!(&term, OwnedTerm::Atom(s) if s == "hello"));
}

#[test]
fn test_serde_ownedterm_visit_bytes() {
    use fasteetf::owned::OwnedTerm;
    struct BytesDeser(Vec<u8>);
    impl<'de> serde_core::Deserializer<'de> for BytesDeser {
        type Error = serde_json::Error;
        fn deserialize_any<V: serde_core::de::Visitor<'de>>(
            self,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            visitor.visit_bytes(&self.0)
        }
        serde_core::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }
    let term: OwnedTerm = serde_core::Deserialize::deserialize(BytesDeser(vec![1, 2, 3])).unwrap();
    assert!(matches!(term, OwnedTerm::Binary(b) if b == vec![1, 2, 3]));
}

#[test]
fn test_serde_ownedterm_visit_byte_buf() {
    use fasteetf::owned::OwnedTerm;
    struct ByteBufDeser(Vec<u8>);
    impl<'de> serde_core::Deserializer<'de> for ByteBufDeser {
        type Error = serde_json::Error;
        fn deserialize_any<V: serde_core::de::Visitor<'de>>(
            self,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            visitor.visit_byte_buf(self.0)
        }
        serde_core::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }
    let term: OwnedTerm =
        serde_core::Deserialize::deserialize(ByteBufDeser(vec![4, 5, 6])).unwrap();
    assert!(matches!(term, OwnedTerm::Binary(b) if b == vec![4, 5, 6]));
}

#[test]
fn test_serde_ownedterm_visit_none() {
    use fasteetf::owned::OwnedTerm;
    struct TypedNoneDeser;
    impl<'de> serde_core::Deserializer<'de> for TypedNoneDeser {
        type Error = serde_json::Error;
        fn deserialize_any<V: serde_core::de::Visitor<'de>>(
            self,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            visitor.visit_none()
        }
        serde_core::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }
    // Deserialize OwnedTerm directly through TypedNoneDeser, which calls
    // visit_none on the OwnedTermVisitor.
    let term: OwnedTerm = serde_core::Deserialize::deserialize(TypedNoneDeser).unwrap();
    // visit_none on OwnedTermVisitor returns Atom("undefined")
    assert!(matches!(&term, OwnedTerm::Atom(s) if s == "undefined"));
}

#[test]
fn test_serde_ownedterm_visit_some() {
    use fasteetf::owned::OwnedTerm;
    // Drive visit_some on the OwnedTermVisitor by deserializing OwnedTerm
    // through a deserializer that calls visit_some.  The visitor delegates
    // back to OwnedTerm::deserialize, which then calls deserialize_any on
    // the inner deserializer (IntDeser → visit_i64).
    struct IntDeser(i64);
    impl<'de> serde_core::Deserializer<'de> for IntDeser {
        type Error = serde_json::Error;
        fn deserialize_any<V: serde_core::de::Visitor<'de>>(
            self,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            visitor.visit_i64(self.0)
        }
        serde_core::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }
    struct SomeDeser(IntDeser);
    impl<'de> serde_core::Deserializer<'de> for SomeDeser {
        type Error = serde_json::Error;
        fn deserialize_any<V: serde_core::de::Visitor<'de>>(
            self,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            visitor.visit_some(self.0)
        }
        serde_core::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }
    let term: OwnedTerm = serde_core::Deserialize::deserialize(SomeDeser(IntDeser(99))).unwrap();
    assert!(matches!(term, OwnedTerm::Int(99)));
}

#[test]
fn test_serde_ownedterm_visit_newtype_struct() {
    use fasteetf::owned::OwnedTerm;
    // visit_newtype_struct delegates to OwnedTerm::deserialize.  The inner
    // deserializer delivers an int.
    struct IntDeser(i64);
    impl<'de> serde_core::Deserializer<'de> for IntDeser {
        type Error = serde_json::Error;
        fn deserialize_any<V: serde_core::de::Visitor<'de>>(
            self,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            visitor.visit_i64(self.0)
        }
        serde_core::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }
    struct NewtypeDeser(IntDeser);
    impl<'de> serde_core::Deserializer<'de> for NewtypeDeser {
        type Error = serde_json::Error;
        fn deserialize_any<V: serde_core::de::Visitor<'de>>(
            self,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            visitor.visit_newtype_struct(self.0)
        }
        serde_core::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }
    let val: OwnedTerm = serde_core::Deserialize::deserialize(NewtypeDeser(IntDeser(99))).unwrap();
    assert!(matches!(val, OwnedTerm::Int(99)));
}

#[test]
fn test_serde_ownedterm_visit_unit() {
    use fasteetf::owned::OwnedTerm;
    // Drive visit_unit directly.
    struct UnitDeser;
    impl<'de> serde_core::Deserializer<'de> for UnitDeser {
        type Error = serde_json::Error;
        fn deserialize_any<V: serde_core::de::Visitor<'de>>(
            self,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            visitor.visit_unit()
        }
        serde_core::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }
    let term: OwnedTerm = serde_core::Deserialize::deserialize(UnitDeser).unwrap();
    assert!(matches!(term, OwnedTerm::List(ref e) if e.is_empty()));
}

#[test]
fn test_serde_ownedterm_visit_bool_true() {
    use fasteetf::owned::OwnedTerm;
    struct BoolDeser(bool);
    impl<'de> serde_core::Deserializer<'de> for BoolDeser {
        type Error = serde_json::Error;
        fn deserialize_any<V: serde_core::de::Visitor<'de>>(
            self,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            visitor.visit_bool(self.0)
        }
        serde_core::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }
    let term: OwnedTerm = serde_core::Deserialize::deserialize(BoolDeser(true)).unwrap();
    assert!(matches!(&term, OwnedTerm::Atom(s) if s == "true"));
}

#[test]
fn test_serde_ownedterm_visit_bool_false() {
    use fasteetf::owned::OwnedTerm;
    struct BoolDeser(bool);
    impl<'de> serde_core::Deserializer<'de> for BoolDeser {
        type Error = serde_json::Error;
        fn deserialize_any<V: serde_core::de::Visitor<'de>>(
            self,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            visitor.visit_bool(self.0)
        }
        serde_core::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }
    let term: OwnedTerm = serde_core::Deserialize::deserialize(BoolDeser(false)).unwrap();
    assert!(matches!(&term, OwnedTerm::Atom(s) if s == "false"));
}

#[test]
fn test_serde_ownedterm_visit_str() {
    use fasteetf::owned::OwnedTerm;
    struct StrDeser(&'static str);
    impl<'de> serde_core::Deserializer<'de> for StrDeser {
        type Error = serde_json::Error;
        fn deserialize_any<V: serde_core::de::Visitor<'de>>(
            self,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            visitor.visit_str(self.0)
        }
        serde_core::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }
    let term: OwnedTerm = serde_core::Deserialize::deserialize(StrDeser("world")).unwrap();
    assert!(matches!(&term, OwnedTerm::Atom(s) if s == "world"));
}

#[test]
fn test_serde_ownedterm_visit_f64() {
    use fasteetf::owned::OwnedTerm;
    struct F64Deser(f64);
    impl<'de> serde_core::Deserializer<'de> for F64Deser {
        type Error = serde_json::Error;
        fn deserialize_any<V: serde_core::de::Visitor<'de>>(
            self,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            visitor.visit_f64(self.0)
        }
        serde_core::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }
    let term: OwnedTerm = serde_core::Deserialize::deserialize(F64Deser(2.5)).unwrap();
    assert!(matches!(term, OwnedTerm::Float(v) if (v - 2.5).abs() < 1e-6));
}

#[test]
fn test_serde_ownedterm_visit_i64_in_range() {
    use fasteetf::owned::OwnedTerm;
    struct I64Deser(i64);
    impl<'de> serde_core::Deserializer<'de> for I64Deser {
        type Error = serde_json::Error;
        fn deserialize_any<V: serde_core::de::Visitor<'de>>(
            self,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            visitor.visit_i64(self.0)
        }
        serde_core::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }
    let term: OwnedTerm = serde_core::Deserialize::deserialize(I64Deser(42)).unwrap();
    assert!(matches!(term, OwnedTerm::Int(42)));
}

#[test]
fn test_serde_ownedterm_visit_u64_in_range() {
    use fasteetf::owned::OwnedTerm;
    struct U64Deser(u64);
    impl<'de> serde_core::Deserializer<'de> for U64Deser {
        type Error = serde_json::Error;
        fn deserialize_any<V: serde_core::de::Visitor<'de>>(
            self,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            visitor.visit_u64(self.0)
        }
        serde_core::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }
    let term: OwnedTerm = serde_core::Deserialize::deserialize(U64Deser(100)).unwrap();
    assert!(matches!(term, OwnedTerm::Int(100)));
}
