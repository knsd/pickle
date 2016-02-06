
use std::io::{Cursor};

extern crate test;
extern crate pickle;

fn main() {
    let buffer = include_bytes!("../pickle");

    println!("{:?}", pickle::unpickle(&mut Cursor::new(&buffer[..])));
    ()
}