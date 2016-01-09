extern crate num;
extern crate byteorder;
#[macro_use] extern crate quick_error;
extern crate from_ascii;
extern crate unicode_names;

#[allow(dead_code)]
pub mod opcode;
#[allow(dead_code)]
pub mod value;
#[allow(dead_code)]
pub mod machine;
mod string;

use std::io::{Read, BufRead};

quick_error! {
    #[derive(Debug)]
    pub enum Error {
        OpCode(err: opcode::Error) {
            from()
        }
        Machine(err: machine::Error) {
            from()
        }
    }
}

pub fn unpickle<R>(rd: &mut R) -> Result<value::Value, Error> where R: Read + BufRead {
    let mut machine = machine::Machine::new();
    loop {
        let opcode = try!(opcode::read_opcode(rd));
        if try!(machine.execute(opcode)) {
            break
        }
    }
    Ok(try!(machine.pop()))
}
