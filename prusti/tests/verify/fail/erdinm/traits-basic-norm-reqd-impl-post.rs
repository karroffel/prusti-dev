extern crate prusti_contracts;

trait Percentage {
    #[ensures="result <= 100"]
    fn get(&self) -> u8;
}

struct Effective {}

impl Percentage for Effective {
    fn get(&self) -> u8 { //~ ERROR postcondition might not hold
        101
    }
}

fn main() {}
