extern crate prusti_contracts;

#[pure]
#[trusted] // pretend to be abstract (bodyless)
fn valid<U>(u: &U) -> bool {
    true
}

#[pure]
fn read<U>(u: &U) -> bool {
    true
}

fn write<U>(u: &mut U) {
}

#[requires="valid(u)"]
fn test<U>(u: &mut U) {
    assert!(valid(u));
    read(u);
    assert!(valid(u));
    write(u);
    assert!(valid(u)); //~ ERROR assert!(..) statement might not hold
}

fn main() {}
