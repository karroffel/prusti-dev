extern crate prusti_contracts;

use std::marker::PhantomData;

struct Even;
struct Odd;

#[invariant="S == Even ~~> self.i % 2 == 0"]
#[invariant="S == Odd  ~~> self.i % 2 != 0"]
struct Int<S> {
    i: i32,
    s: PhantomData<S>,
}

impl<A> Int<A> {
    #[requires="A == Even ~~> i % 2 == 0"]
    #[requires="A == Odd  ~~> i % 2 != 0"]
    fn new(i: i32) -> Int<A> {
        Int {
            i,
            s: PhantomData,
        }
    }

    fn test_incr2(&mut self) { //~ ERROR postcondition might not hold
        self.i += 3;
    }

    fn test_plus2(self) -> Self { //~ ERROR postcondition might not hold
        Int {
            i: self.i + 3,
            s: PhantomData,
        }
    }

    // non-negative because modulo doesn't like negative numbers (currently)
    #[requires="self.i >= 0"]
    fn test_double(self) -> Int<Even> {
        Int::new(self.i * 3) //~ ERROR precondition might not hold
    }
}

fn test1(int: &mut Int<Even>) {
    assert!(int.i % 2 != 0); //~ ERROR assert!(..) statement might not hold
}

fn test2(int: &mut Int<Odd>) {
    assert!(int.i % 2 == 0); //~ ERROR assert!(..) statement might not hold
}

#[requires="i % 2 != 0"]
fn test3(i: i32) -> Int<Even> {
    Int::new(i) //~ ERROR precondition might not hold
}

#[requires="i % 2 == 0"]
fn test4(i: i32) -> Int<Odd> {
    Int::new(i) //~ ERROR precondition might not hold
}

fn main() {}
