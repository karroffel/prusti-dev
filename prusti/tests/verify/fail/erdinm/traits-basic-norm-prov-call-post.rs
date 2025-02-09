extern crate prusti_contracts;

trait Percentage {
    #[ensures="result <= 100"]
    fn get(&self) -> u8 {
        100
    }
}

fn test<T: Percentage>(t: &T) {
    let p = t.get();
    assert!(p <= 99); //~ ERROR assert!(..) statement might not hold
}

fn main() {}
