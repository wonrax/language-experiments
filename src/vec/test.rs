// Hacky way to alias super because `use super as vec` won't compile
// https://github.com/rust-lang/rust/issues/29036
mod vec {
    pub use super::super::*;
}

#[test]
fn test_new_vec() {
    let v: vec::Vec<i32> = vec::new();
    assert_eq!(v.len(), 0);
    assert_eq!(v.index(1), None);
    assert_eq!(v.index(0), None);
}
