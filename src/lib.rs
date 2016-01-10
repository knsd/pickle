extern crate num;
extern crate byteorder;
#[macro_use] extern crate quick_error;
extern crate from_ascii;
extern crate unicode_names;

#[allow(dead_code)]
pub mod opcodes;
#[allow(dead_code)]
pub mod value;
#[allow(dead_code)]
pub mod machine;
mod string;

// use std::io::{Read, BufRead};

// quick_error! {
//     #[derive(Debug)]
//     pub enum Error {
//         Machine(err: machine::Error) {
//             from()
//         }
//     }
// }

// pub fn unpickle<R>(rd: &mut R) -> Result<value::Value, Error> where R: Read + BufRead {
//     let mut machine = machine::Machine::new();
//     loop {
//         let opcode = try!(opcode::read_opcode(rd));
//         if try!(machine.execute(opcode)) {
//             break
//         }
//     }
//     Ok(try!(machine.pop()))
// }

// #[cfg(test)]
// mod tests {
//     use std::io::{Cursor};

//     use num::{FromPrimitive};

//     use super::{unpickle};
//     use super::value::{Value};

//     macro_rules! t {
//         ($buffer: expr, $pat:pat, $result:expr) => ({
//             match unpickle(&mut Cursor::new(&$buffer[..])) {
//                 Ok($pat) => $result,
//                 other => {
//                     println!("ERROR {:?}", other);
//                     assert!(false)
//                 },
//             }
//         })
//     }

//     macro_rules! n {
//         ($x: expr) => ({Value::Int(FromPrimitive::from_isize($x).unwrap())})
//     }

//     #[test]
//     fn test_number() {
//         t!(b"I3\n.", n, assert_eq!(n, n!(3)));
//         t!(b"K\x03.", n, assert_eq!(n, n!(3)));
//         t!(b"\x80\x02K\x03.", n, assert_eq!(n, n!(3)));
//     }

//     #[test]
//     fn test() {
//         t!(b"(lp0\nI1\naI2\naI3\naI4\na.", Value::List(v), assert_eq!(v, vec!(n!(1), n!(2), n!(3), n!(4))))
//     }
// }
