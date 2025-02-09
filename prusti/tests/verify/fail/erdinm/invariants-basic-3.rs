extern crate prusti_contracts;

// postcondition (&mut arg) inhale

//#[invariant="self.value <= 100"]
struct Percentage {
    value: u8,
}

impl Percentage {
    fn incr(&mut self) {
        if self.value < 100 {
            self.value += 1;
        }
    }
}

#[requires="x <= 100"]
fn test(x: u8) {
    let mut perc = Percentage { value: x };
    perc.incr();
    assert!(perc.value <= 100); //~ ERROR assert!(..) statement might not hold
}

fn main() {}
