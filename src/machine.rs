// Copyright (c) 2016 Fedor Gogolev <knsd@knsd.net>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::io::{Read, BufRead, Error as IoError, ErrorKind};
use std::string::{FromUtf8Error};
use std::cell::{RefCell};
use std::rc::{Rc};

use num::{Zero};
use num::bigint::{BigInt, ToBigInt, Sign};
use byteorder::{ReadBytesExt, LittleEndian, BigEndian, Error as ByteorderError};
use from_ascii::{FromAscii, ParseIntError, ParseFloatError};

use string::{unescape, Error as UnescapeError};
use value::{Value};

use opcodes::*;

quick_error! {
    #[derive(Debug)]
    pub enum Error {
        EmptyMarker
        StackTooSmall
        EmptyStack
        InvalidValueOnStack
        InvalidGetValue
        InvalidPutValue
        NotImplemented

        Read(err: ByteorderError) {
            from()
        }
        Io(err: IoError) {
            from()
        }
        UnknownOpcode(opcode: u8) {}

        InvalidInt {
            from(ParseIntError)
        }
        InvalidLong
        InvalidFloat {
            from(ParseFloatError)
        }

        InvalidString {
            from(FromUtf8Error)
        }
        UnescapeError(err: UnescapeError) {
            from()
        }

        InvalidProto(proto: u8)
        NegativeLength {}

        #[doc(hidden)]
        __Nonexhaustive
    }
}

#[derive(Debug, PartialEq)]
pub enum BooleanOrInt {
    Boolean(bool),
    Int(i64),
}

macro_rules! rc {
    ($term: expr) => (Rc::new(RefCell::new($term)))
}

fn read_exact<R>(rd: &mut R, mut buf: &mut [u8]) -> Result<(), IoError> where R: Read {
    while !buf.is_empty() {
        match rd.read(buf) {
            Ok(0) => break,
            Ok(n) => { let tmp = buf; buf = &mut tmp[n..]; }
            Err(ref e) if e.kind() == ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    if !buf.is_empty() {
        Err(IoError::new(ErrorKind::Other,
                       "failed to fill whole buffer"))
    } else {
        Ok(())
    }
}

fn read_until_newline<R>(rd: &mut R) -> Result<Vec<u8>, Error> where R: Read + BufRead {
    let mut buf = Vec::new();
    try!(rd.read_until('\n' as u8, &mut buf));

    // Skip last symbol — \n
    match buf.split_last() {
        Some((&b'\n', init)) => Ok(init.to_vec()),
        _ => return Err(Error::InvalidString),
    }
}

fn read_decimal_int<R>(rd: &mut R) -> Result<BooleanOrInt, Error> where R: Read + BufRead {
    let s = try!(read_until_newline(rd));
    let val = match &s[..] {
        b"00" => BooleanOrInt::Boolean(false),
        b"01" => BooleanOrInt::Boolean(true),
        _ => BooleanOrInt::Int(try!(i64::from_ascii(&s)))
    };
    Ok(val)
}

fn read_decimal_long<R>(rd: &mut R) -> Result<BigInt, Error> where R: Read + BufRead {
    let s = try!(read_until_newline(rd));
    let init = match s.split_last() {
        None => return Err(Error::InvalidString),
        Some((&b'L', init)) => init,
        Some(_) => &s[..],
    };

    match BigInt::parse_bytes(&init, 10) {
        Some(i) => Ok(i),
        None => Err(Error::InvalidLong)
    }
}


fn read_long<R>(rd: &mut R, length: usize) -> Result<BigInt, Error> where R: Read + BufRead {
    let mut buf = vec![0; length];
    try!(read_exact(rd, buf.as_mut()));

    let mut n = BigInt::from_bytes_le(Sign::Plus, &buf);

    let last = match buf.last_mut() {
        None => return Err(Error::InvalidLong),
        Some(last) => last,
    };

    if *last > 127 {
        n = n - (1.to_bigint().unwrap() << (length * 8))
    }

    Ok(n)
}

pub struct Machine {
    stack: Vec<Value>,
    memo: Vec<Value>,
    marker: Option<usize>,
}

impl Machine {
    pub fn new() -> Self {
        Machine {
            stack: Vec::new(),
            memo: vec![Value::None],
            marker: None,
        }
    }

    fn split_off(&mut self) -> Result<Vec<Value>, Error> {
        let at = match self.marker {
            None => return Err(Error::EmptyMarker),
            Some(marker) => marker,
        };

        if at > self.stack.len() {
            return Err(Error::StackTooSmall);
        }

        Ok(self.stack.split_off(at))
    }

    pub fn pop(&mut self) -> Result<Value, Error> {
        match self.stack.pop() {
            None => return Err(Error::EmptyStack),
            Some(value) => Ok(value),
        }
    }

    fn handle_get(&mut self, i: usize) -> Result<(), Error> {
        let value = match self.memo.get(i) {
            None => return Err(Error::InvalidGetValue),
            Some(ref v) => (*v).clone(),
        };
        self.stack.push(value);
        Ok(())
    }

    fn handle_put(&mut self, i: usize) -> Result<(), Error> {
        let value = match self.stack.last() {
            None => return Err(Error::EmptyStack),
            Some(ref v) => (*v).clone(),
        };
        let len = self.memo.len();
        if len != i {
            return Err(Error::InvalidPutValue)
        }
        self.memo.push(value);
        Ok(())
    }

    pub fn execute<R>(&mut self, rd: &mut R) -> Result<bool, Error> where R: Read + BufRead {
        macro_rules! ensure_not_negative {
            ($n: expr) => ({
                if $n < Zero::zero() {
                    return Err(Error::NegativeLength)
                }
            })
        }

        let marker = try!(rd.read_u8());
        match marker {
            PROTO => {
                let version = try!(rd.read_u8());
                if version < 2 {
                    return Err(Error::InvalidProto(version))
                }
            },
            STOP => return Ok(true),

            INT => {
                self.stack.push(match try!(read_decimal_int(rd)) {
                    BooleanOrInt::Boolean(v) => Value::Bool(v),
                    BooleanOrInt::Int(v) => Value::Long(BigInt::from(v)),
                })
            },
            BININT => self.stack.push(Value::Int(try!(rd.read_i32::<LittleEndian>()) as isize)),
            BININT1 => self.stack.push(Value::Int(try!(rd.read_u8()) as isize)),
            BININT2 => self.stack.push(Value::Int(try!(rd.read_u16::<LittleEndian>()) as isize)),
            LONG => self.stack.push(Value::Long(BigInt::from(try!(read_decimal_long(rd))))),
            LONG1 => {
                let length = try!(rd.read_u8());
                self.stack.push(Value::Long(BigInt::from(try!(read_long(rd, length as usize)))))
            }
            LONG4 => {
                let length = try!(rd.read_i32::<LittleEndian>());
                self.stack.push(Value::Long(BigInt::from(try!(read_long(rd, length as usize)))))
            }

            STRING => self.stack.push(Value::String(try!(unescape(&try!(read_until_newline(rd)), false)))),
            BINSTRING => {
                let length = try!(rd.read_i32::<LittleEndian>());
                ensure_not_negative!(length);

                let mut buf = vec![0; length as usize];
                try!(read_exact(rd, &mut buf));
                self.stack.push(Value::String(buf))
            },
            SHORT_BINSTRING => {
                let length = try!(rd.read_u8());
                let mut buf = vec![0; length as usize];
                try!(read_exact(rd, &mut buf));
                self.stack.push(Value::String(buf))
            },

            NONE => self.stack.push(Value::None),
            NEWTRUE => self.stack.push(Value::Bool(true)),
            NEWFALSE => self.stack.push(Value::Bool(false)),

            UNICODE => {
                let buf = try!(unescape(&try!(read_until_newline(rd)), true));
                self.stack.push(Value::Unicode(try!(String::from_utf8(buf))))
            },
            BINUNICODE => {
                let length = try!(rd.read_i32::<LittleEndian>());
                ensure_not_negative!(length);
                let mut buf = vec![0; length as usize];
                try!(read_exact(rd, buf.as_mut()));
                self.stack.push(Value::Unicode(try!(String::from_utf8(buf))))
            },

            FLOAT => {
                let s = try!(read_until_newline(rd));
                self.stack.push(Value::Float(try!(f64::from_ascii(&s))))
            },
            BINFLOAT => {
                self.stack.push(Value::Float(try!(rd.read_f64::<BigEndian>())))
            },

            EMPTY_LIST => {
                self.stack.push(Value::List(rc!(Vec::new())))
            },
            APPEND => {
                let v = try!(self.pop());
                match self.stack.last_mut() {
                    None => return Err(Error::EmptyStack),
                    Some(&mut Value::List(ref mut list)) => (*list.borrow_mut()).push(v),
                    _ => return Err(Error::InvalidValueOnStack),
                }
            },
            APPENDS => {
                let values = try!(self.split_off());
                match self.stack.last_mut() {
                    None => return Err(Error::EmptyStack),
                    Some(&mut Value::List(ref mut list)) => (*list.borrow_mut()).extend(values),
                    _ => return Err(Error::InvalidValueOnStack),
                }
            },
            LIST => {
                let values = try!(self.split_off());
                self.stack.push(Value::List(rc!(values)));
            },

            EMPTY_TUPLE => self.stack.push(Value::Tuple(rc!(Vec::new()))),
            TUPLE => {
                let values = try!(self.split_off());
                self.stack.push(Value::Tuple(rc!(values)));
            },
            TUPLE1 => {
                let v1 = try!(self.pop());
                self.stack.push(Value::Tuple(rc!(vec![v1])))
            },
            TUPLE2 => {
                let v1 = try!(self.pop());
                let v2 = try!(self.pop());
                self.stack.push(Value::Tuple(rc!(vec![v1, v2])))
            },
            TUPLE3 => {
                let v1 = try!(self.pop());
                let v2 = try!(self.pop());
                let v3 = try!(self.pop());
                self.stack.push(Value::Tuple(rc!(vec![v1, v2, v3])))
            }

            EMPTY_DICT => self.stack.push(Value::Dict(rc!(Vec::new()))),
            DICT => {
                let mut values = try!(self.split_off());
                let mut dict = Vec::new();

                for i in 0 .. values.len() / 2 { // TODO: Check panic
                    let key = values.remove(2 * i);
                    let value = values.remove(2 * i + 1);
                    dict.push((key, value));
                }
                self.stack.push(Value::Dict(rc!(dict)));
            },
            SETITEM => {
                let value = try!(self.pop());
                let key = try!(self.pop());
                match self.stack.last_mut() {
                    None => return Err(Error::EmptyStack),
                    Some(&mut Value::Dict(ref mut dict)) => (*dict.borrow_mut()).push((key, value)),
                    _ => return Err(Error::InvalidValueOnStack),
                }
            },
            SETITEMS => {
                let mut values = try!(self.split_off());

                match self.stack.last_mut() {
                    None => return Err(Error::EmptyStack),
                    Some(&mut Value::Dict(ref mut dict_ref)) => {
                        for i in 0 .. values.len() / 2 { // TODO: Check panic
                                let key = values.remove(2 * i);
                                let value = values.remove(2 * i + 1);
                                (*dict_ref.borrow_mut()).push((key, value));
                            }
                    },
                    _ => return Err(Error::InvalidValueOnStack),
                }
            },

            POP => {
                try!(self.pop());
            },
            DUP => {
                let value = match self.stack.last() {
                    None => return Err(Error::EmptyStack),
                    Some(ref v) => (*v).clone(),
                };
                self.stack.push(value)
            },
            MARK => {
                self.marker = Some(self.stack.len())
            },
            POP_MARK => {
                try!(self.split_off());
            },

            GET => {
                let n = match try!(read_decimal_int(rd)) {
                    BooleanOrInt::Int(n) => n,
                    BooleanOrInt::Boolean(false) => 0,
                    BooleanOrInt::Boolean(true) => 1,
                };
                ensure_not_negative!(n);
                try!(self.handle_get(n as usize))
            }
            BINGET => {
                try!(self.handle_get(try!(rd.read_u8()) as usize))
            }
            LONG_BINGET => {
                let n = try!(rd.read_i32::<LittleEndian>());
                ensure_not_negative!(n);
                try!(self.handle_get(n as usize))
            }

            PUT => {
                let n = match try!(read_decimal_int(rd)) {
                    BooleanOrInt::Int(n) => n,
                    BooleanOrInt::Boolean(false) => 0,
                    BooleanOrInt::Boolean(true) => 1,
                };
                ensure_not_negative!(n);
                try!(self.handle_put(n as usize))
            }
            BINPUT => {
                try!(self.handle_put(try!(rd.read_u8()) as usize))
            }
            LONG_BINPUT => {
                let n = try!(rd.read_i32::<LittleEndian>());
                ensure_not_negative!(n);
                try!(self.handle_put(n as usize))
            }

            c => return Err(Error::UnknownOpcode(c)),
        }
        Ok(false)
    }
}
