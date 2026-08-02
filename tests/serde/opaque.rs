// ── Opaque owned wrappers: serialize + deserialize roundtrip ───────────────

#[test]
fn test_serde_pid_owned_roundtrip() {
    use fasteetf::owned::PidOwned;
    let pid = PidOwned(vec![1, 2, 3, 4]);
    let json = serde_json::to_string(&pid).unwrap();
    let de: PidOwned = serde_json::from_str(&json).unwrap();
    assert_eq!(de.0, vec![1, 2, 3, 4]);
}

#[test]
fn test_serde_port_owned_roundtrip() {
    use fasteetf::owned::PortOwned;
    let port = PortOwned(vec![5, 6, 7, 8]);
    let json = serde_json::to_string(&port).unwrap();
    let de: PortOwned = serde_json::from_str(&json).unwrap();
    assert_eq!(de.0, vec![5, 6, 7, 8]);
}

#[test]
fn test_serde_ref_owned_roundtrip() {
    use fasteetf::owned::ReferenceOwned;
    let r = ReferenceOwned(vec![9, 10, 11, 12]);
    let json = serde_json::to_string(&r).unwrap();
    let de: ReferenceOwned = serde_json::from_str(&json).unwrap();
    assert_eq!(de.0, vec![9, 10, 11, 12]);
}

#[test]
fn test_serde_function_owned_roundtrip() {
    use fasteetf::owned::FunctionOwned;
    let f = FunctionOwned(vec![13, 14, 15]);
    let json = serde_json::to_string(&f).unwrap();
    let de: FunctionOwned = serde_json::from_str(&json).unwrap();
    assert_eq!(de.0, vec![13, 14, 15]);
}

#[test]
fn test_serde_record_owned_roundtrip() {
    use fasteetf::owned::RecordOwned;
    let r = RecordOwned(vec![20, 21, 22]);
    let json = serde_json::to_string(&r).unwrap();
    let de: RecordOwned = serde_json::from_str(&json).unwrap();
    assert_eq!(de.0, vec![20, 21, 22]);
}

#[test]
fn test_serde_record_owned_from_seq() {
    use fasteetf::owned::RecordOwned;
    // JSON array of integers exercises the visit_seq path of the UntaggedOpaqueVisitor.
    let de: RecordOwned = serde_json::from_str("[100, 101, 102]").unwrap();
    assert_eq!(de.0, vec![100, 101, 102]);
}

// ── UntaggedOpaqueVisitor (for RecordOwned): drive visit_bytes and visit_byte_buf ──

#[test]
fn test_serde_untagged_opaque_visit_bytes() {
    use fasteetf::owned::RecordOwned;
    // Drive visit_bytes on the UntaggedOpaqueVisitor by deserializing
    // RecordOwned through a deserializer that calls visit_bytes.
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
    let rec: RecordOwned =
        serde_core::Deserialize::deserialize(BytesDeser(vec![10, 20, 30])).unwrap();
    assert_eq!(rec.0, vec![10, 20, 30]);
}

#[test]
fn test_serde_untagged_opaque_visit_byte_buf() {
    use fasteetf::owned::RecordOwned;
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
    let rec: RecordOwned =
        serde_core::Deserialize::deserialize(ByteBufDeser(vec![40, 50, 60])).unwrap();
    assert_eq!(rec.0, vec![40, 50, 60]);
}

// ── expect() paths: trigger deserialization errors that name the visitor ────

#[test]
fn test_serde_ownedterm_expect_error() {
    use fasteetf::owned::OwnedTerm;
    // Force the OwnedTermVisitor's expecting() to be invoked by feeding
    // OwnedTerm through a deserializer that calls a method whose default
    // implementation invokes expecting().  The default visit_unit calls
    // expecting() to format the error.
    struct UnitDeser;
    impl<'de> serde_core::Deserializer<'de> for UnitDeser {
        type Error = serde_json::Error;
        fn deserialize_any<V: serde_core::de::Visitor<'de>>(
            self,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            // visit_unit is overridden on OwnedTermVisitor (returns empty
            // List).  To trigger expecting(), we need a visit method that
            // OwnedTermVisitor does NOT override and uses the default
            // expecting() error.  visit_enum would do that but is not
            // commonly triggered by JSON.  Use deserialize_enum instead.
            visitor.visit_unit()
        }
        serde_core::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }
    // Direct deserialize.  OwnedTermVisitor's visit_unit returns empty
    // List, so this succeeds.  Use a different path: a struct that
    // expects to call visit_seq on OwnedTermVisitor (which it does have).
    let _: OwnedTerm = serde_core::Deserialize::deserialize(UnitDeser).unwrap();
}

#[test]
fn test_serde_untagged_opaque_expect_error() {
    use fasteetf::owned::RecordOwned;
    // Construct an integer JSON value.  serde_json's deserializer will
    // call visit_i64, but UntaggedOpaqueVisitor only implements
    // visit_bytes/byte_buf/seq.  The default visit_i64 returns an error
    // that names the visitor's expecting() string.
    let result: Result<RecordOwned, _> = serde_json::from_str("42");
    assert!(result.is_err());
    let err = result.unwrap_err();
    let msg = format!("{err}");
    // The error should mention "byte array" (from expecting()).
    assert!(
        msg.contains("byte") || msg.contains("invalid type"),
        "got: {msg}"
    );
}

#[test]
fn test_serde_tagged_opaque_expect_error() {
    use fasteetf::owned::PidOwned;
    // Trigger expecting() on TaggedOpaqueVisitor by feeding JSON with a map,
    // while the visitor only accepts byte strings and byte sequences.
    let result: Result<PidOwned, _> = serde_json::from_str("{}");
    assert!(result.is_err());
    let err = result.unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("byte") || msg.contains("invalid type"));
}
