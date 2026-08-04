use crate::tags::*;

#[kani::proof]
#[kani::unwind(500)]
fn tags_are_distinct() {
    let tags = [
        ATOM_UTF8_EXT,
        SMALL_ATOM_UTF8_EXT,
        SMALL_INTEGER_EXT,
        INTEGER_EXT,
        SMALL_BIG_EXT,
        LARGE_BIG_EXT,
        NEW_FLOAT_EXT,
        FLOAT_EXT,
        SMALL_TUPLE_EXT,
        LARGE_TUPLE_EXT,
        NIL_EXT,
        STRING_EXT,
        LIST_EXT,
        MAP_EXT,
        BINARY_EXT,
        BIT_BINARY_EXT,
        PID_EXT,
        NEW_PID_EXT,
        PORT_EXT,
        NEW_PORT_EXT,
        V4_PORT_EXT,
        NEW_REFERENCE_EXT,
        NEWER_REFERENCE_EXT,
        NEW_FUN_EXT,
        EXPORT_EXT,
        RECORD_EXT,
        COMPRESSED,
        ATOM_CACHE_REF,
        LOCAL_EXT,
    ];
    for i in 0..tags.len() {
        for j in (i + 1)..tags.len() {
            assert_ne!(tags[i], tags[j]);
        }
    }
}
