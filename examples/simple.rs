use std::io::{Cursor};

extern crate pickle;

fn main() {
    let buffer = include_bytes!("../pickle");

    // let start = time::precise_time_ns();
    pickle::machine::unpickle(&mut Cursor::new(&buffer[..]));
    // println!("done in {:.6}", (time::precise_time_ns() - start)  as f64 / 1000000000 as f64);
    ()
}