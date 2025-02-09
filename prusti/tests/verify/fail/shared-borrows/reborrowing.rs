extern crate prusti_contracts;

#[ensures="*result == old(*x)"]
pub fn reborrow(x: &u32) -> &u32 {
    assert!(false); //~ ERROR assert!(..) statement might not hold
    x
}

#[ensures="*result == old(*x)"]
#[ensures="false"]
pub fn reborrow2(x: &u32) -> &u32 { //~ ERROR postcondition might not hold.
    x
}

pub fn test1() {
    let mut a = 5;
    let x = &a;
    let y = reborrow(x);
    assert!(a == 5);
    assert!(*x == 5);
    assert!(*y == 5);
    assert!(a == 5);
    a = 6;
    assert!(a == 6);
    assert!(false); //~ ERROR assert!(..) statement might not hold
}

fn main() {
}

