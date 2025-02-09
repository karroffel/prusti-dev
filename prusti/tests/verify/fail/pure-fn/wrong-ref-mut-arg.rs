#![feature(nll)]
#![feature(box_patterns)]
#![feature(box_syntax)]
#![feature(never_type)]
#![allow(unconditional_recursion)]

extern crate prusti_contracts;

use std::borrow::BorrowMut;

struct List {
    value: u32,
    next: Option<Box<List>>,
}

#[pure]
fn lookup(head: &mut List, index: usize) -> u32 {
    if index == 0 {
        head.value
    } else {
        match head.next {
            Some(box ref mut tail) => lookup(tail, index - 1),
            None => unreachable!() //~ ERROR might be reachable
        }
    }
}

#[pure]
fn len(head: &mut List) -> usize {
    match head.next {
        None => 1,
        Some(box ref mut tail) => 1 + len(tail)
    }
}

fn diverging() -> ! {
    diverging()
}

fn prepend_list(x: u32, tail: List, check: bool) -> List {
    let mut result = List {
        value: x,
        next: Some(Box::new(tail)),
    };
    if check {
        assert!(lookup(&mut result, 0) == 123); //~ ERROR assert!(..) statement might not hold
        diverging()
    }
    result
}


fn main() {}
