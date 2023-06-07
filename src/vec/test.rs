use super::Vec;

#[test]
fn test_new_vec() {
    let v: Vec<i32> = Vec::new();
    assert_eq!(v.len(), 0);
    assert_eq!(v.index(1), None);
    assert_eq!(v.index(0), None);
}
