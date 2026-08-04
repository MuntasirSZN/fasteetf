use crate::error::Needed;

#[kani::proof]
fn needed_size_matches_is_exact() {
    let n: usize = kani::any();
    let size = Needed::Size(n);
    assert!(size.is_exact());
    assert_eq!(size.size(), Some(n));

    let unknown = Needed::Unknown;
    assert!(!unknown.is_exact());
    assert_eq!(unknown.size(), None);
}
