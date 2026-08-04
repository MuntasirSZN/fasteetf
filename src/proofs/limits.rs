use crate::limits::Limits;

#[kani::proof]
fn limits_defaults_are_positive() {
    let d = Limits::default();
    assert!(d.max_binary_size > 0);
    assert!(d.max_bit_binary_size > 0);
    assert!(d.max_list_len > 0);
    assert!(d.max_map_len > 0);
    assert!(d.max_atom_len > 0);
    assert!(d.max_tuple_arity > 0);
    assert!(d.max_string_len > 0);
    assert!(d.max_reference_words > 0);
    assert!(d.max_depth > 0);
    assert!(d.max_fun_size > 0);
    assert!(d.max_bignum_size > 0);
    assert_eq!(d.max_reference_words, 5);
    assert_eq!(d.max_depth, 128);
    assert!(d.expand_string_ext_to_list);
}

#[kani::proof]
fn limits_profiles_are_ordered() {
    let d = Limits::default();
    let e = Limits::embedded();
    let r = Limits::relaxed();
    assert!(e.max_binary_size <= d.max_binary_size && d.max_binary_size <= r.max_binary_size);
    assert!(
        e.max_bit_binary_size <= d.max_bit_binary_size
            && d.max_bit_binary_size <= r.max_bit_binary_size
    );
    assert!(e.max_list_len <= d.max_list_len && d.max_list_len <= r.max_list_len);
    assert!(e.max_map_len <= d.max_map_len && d.max_map_len <= r.max_map_len);
    assert!(e.max_atom_len <= d.max_atom_len && d.max_atom_len <= r.max_atom_len);
    assert!(e.max_tuple_arity <= d.max_tuple_arity && d.max_tuple_arity <= r.max_tuple_arity);
    assert!(e.max_string_len <= d.max_string_len && d.max_string_len <= r.max_string_len);
    assert!(
        e.max_reference_words <= d.max_reference_words
            && d.max_reference_words <= r.max_reference_words
    );
    assert!(e.max_depth <= d.max_depth && d.max_depth <= r.max_depth);
    assert!(e.max_fun_size <= d.max_fun_size && d.max_fun_size <= r.max_fun_size);
    assert!(e.max_bignum_size <= d.max_bignum_size && d.max_bignum_size <= r.max_bignum_size);
}
