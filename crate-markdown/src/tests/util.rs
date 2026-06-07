//! tests for the util module

use crate::prelude::*;

#[test]
fn test_intersects() {
    let s1 = Span { start: 0, end: 10 };
    let s2 = Span { start: 5, end: 15 };
    let s3 = Span { start: 10, end: 20 };
    let s4 = Span { start: 15, end: 25 };

    assert!(s1.intersects(s2));
    assert!(s2.intersects(s1));
    assert!(!s1.intersects(s3));
    assert!(!s3.intersects(s1));
    assert!(!s1.intersects(s4));
    assert!(s2.intersects(s3));
}
