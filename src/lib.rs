// Copyright (c) 2016 Fedor Gogolev <knsd@knsd.net>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]

extern crate num;
extern crate byteorder;
#[macro_use] extern crate quick_error;
extern crate from_ascii;
extern crate unicode_names;

pub mod opcodes;
pub mod value;
pub mod machine;
mod string;

#[cfg(test)]
mod tests {
    use std::io::{Cursor};

    use num::{FromPrimitive};

    use super::{unpickle};
    use super::value::{Value};

    macro_rules! t {
        ($buffer: expr, $pat:pat, $result:expr) => ({
            match unpickle(&mut Cursor::new(&$buffer[..])) {
                Ok($pat) => $result,
                other => {
                    println!("ERROR {:?}", other);
                    assert!(false)
                },
            }
        })
    }

    macro_rules! i {
        ($x: expr) => ({Value::Int($x)})
    }

    macro_rules! n {
        ($x: expr) => ({Value::Long(FromPrimitive::from_isize($x).unwrap())})
    }

    #[test]
    fn test_number() {
        t!(b"I3\n.", n, assert_eq!(n, n!(3)));
        t!(b"K\x03.", n, assert_eq!(n, i!(3)));
        t!(b"\x80\x02K\x03.", n, assert_eq!(n, i!(3)));
    }

    #[test]
    fn test() {
        t!(b"(lp0\nI1\naI2\naI3\naI4\na.", Value::List(v), assert_eq!(v, vec!(n!(1), n!(2), n!(3), n!(4))));
    }
}
