use std::collections::{HashMap};

use num::bigint::{BigInt};

use opcode::{OpCode, BooleanOrInt};
use value::{Value};

quick_error! {
    #[derive(Debug)]
    pub enum Error {
        EmptyMarker
        StackTooSmall
        EmptyStack
        InvalidValueOnStack
        InvalidMemoValue
        NotImplemented
    }
}

pub struct Machine {
    stack: Vec<Value>,
    memo: HashMap<usize, Value>,
    marker: Option<usize>,
}

impl Machine {
    fn new() -> Self {
        Machine {
            stack: Vec::new(),
            memo: HashMap::new(),
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

    fn pop(&mut self) -> Result<Value, Error> {
        match self.stack.pop() {
            None => return Err(Error::EmptyStack),
            Some(value) => Ok(value),
        }
    }

    fn handle_get(&mut self, i: usize) -> Result<(), Error> {
        let value = match self.memo.get(&i) {
            None => return Err(Error::InvalidMemoValue),
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
        self.memo.insert(i, value);
        Ok(())
    }

    pub fn execute(&mut self, opcode: OpCode) -> Result<bool, Error> {
        match opcode {
            OpCode::Proto(_) => (),
            OpCode::Stop => return Ok(true),

            OpCode::Int(value) => {
                self.stack.push(match value {
                    BooleanOrInt::Boolean(v) => Value::Bool(v),
                    BooleanOrInt::Int(v) => Value::Int(BigInt::from(v)),
                })
            },
            OpCode::BinInt(i) => self.stack.push(Value::Int(BigInt::from(i))),
            OpCode::BinInt1(i) => self.stack.push(Value::Int(BigInt::from(i))),
            OpCode::BinInt2(i) => self.stack.push(Value::Int(BigInt::from(i))),
            OpCode::Long(i) => self.stack.push(Value::Int(BigInt::from(i))),
            OpCode::Long1(i) => self.stack.push(Value::Int(BigInt::from(i))),
            OpCode::Long4(i) => self.stack.push(Value::Int(BigInt::from(i))),

            OpCode::String(s) => self.stack.push(Value::String(s)),
            OpCode::BinString(s) => self.stack.push(Value::String(s)),
            OpCode::ShortBinString(s) => self.stack.push(Value::String(s)),

            OpCode::None => self.stack.push(Value::None),

            OpCode::NewTrue => self.stack.push(Value::Bool(true)),
            OpCode::NewFalse => self.stack.push(Value::Bool(false)),

            OpCode::Unicode(s) => self.stack.push(Value::Unicode(s)),
            OpCode::BinUnicode(s) => self.stack.push(Value::Unicode(s)),

            OpCode::Float(i) => self.stack.push(Value::Float(i)),
            OpCode::BinFloat(i) => self.stack.push(Value::Float(i)),

            OpCode::EmptyList => self.stack.push(Value::List(Vec::new())),
            OpCode::Append => {
                let v = try!(self.pop());
                match self.stack.last_mut() {
                    None => return Err(Error::EmptyStack),
                    Some(&mut Value::List(ref mut list)) => list.push(v),
                    _ => return Err(Error::InvalidValueOnStack),
                }
            },
            OpCode::Appends => {
                let values = try!(self.split_off());
                match self.stack.last_mut() {
                    None => return Err(Error::EmptyStack),
                    Some(&mut Value::List(ref mut list)) => {
                        list.extend(values);
                    },
                    _ => return Err(Error::InvalidValueOnStack),
                }
            },
            OpCode::List => {
                let values = try!(self.split_off());
                self.stack.push(Value::List(values));
            },

            OpCode::EmptyTuple => self.stack.push(Value::Tuple(Vec::new())),
            OpCode::Tuple => {
                let values = try!(self.split_off());
                self.stack.push(Value::Tuple(values));
            },
            OpCode::Tuple1 => {
                let v1 = try!(self.pop());
                self.stack.push(Value::Tuple(vec![v1]))
            },
            OpCode::Tuple2 => {
                let v1 = try!(self.pop());
                let v2 = try!(self.pop());
                self.stack.push(Value::Tuple(vec![v1, v2]))
            },
            OpCode::Tuple3 => {
                let v1 = try!(self.pop());
                let v2 = try!(self.pop());
                let v3 = try!(self.pop());
                self.stack.push(Value::Tuple(vec![v1, v2, v3]))
            }

            OpCode::EmptyDict => self.stack.push(Value::Dict(Vec::new())),
            OpCode::Dict => {
                let mut values = try!(self.split_off());
                let mut dict = Vec::new();

                for i in 0 .. values.len() / 2 { // TODO: Check panic
                    let key = values.remove(2 * i);
                    let value = values.remove(2 * i + 1);
                    dict.push((key, value));
                }
                self.stack.push(Value::Dict(dict));
            },
            OpCode::SetItem => {
                let value = try!(self.pop());
                let key = try!(self.pop());
                match self.stack.last_mut() {
                    None => return Err(Error::EmptyStack),
                    Some(&mut Value::Dict(ref mut dict)) => dict.push((key, value)),
                    _ => return Err(Error::InvalidValueOnStack),
                }
            },
            OpCode::SetItems => {
                let mut values = try!(self.split_off());

                match self.stack.last_mut() {
                    None => return Err(Error::EmptyStack),
                    Some(&mut Value::Dict(ref mut dict)) => {
                        for i in 0 .. values.len() / 2 { // TODO: Check panic
                            let key = values.remove(2 * i);
                            let value = values.remove(2 * i + 1);
                            dict.push((key, value));
                        }
                    },
                    _ => return Err(Error::InvalidValueOnStack),
                }
            },

            OpCode::Pop => {
                try!(self.pop());
            },
            OpCode::Dup => {
                let value = match self.stack.last() {
                    None => return Err(Error::EmptyStack),
                    Some(ref v) => (*v).clone(),
                };
                self.stack.push(value)
            },
            OpCode::Mark => {
                self.marker = Some(self.stack.len())
            },
            OpCode::PopMark => {
                try!(self.split_off());
            },

            OpCode::Get(i) => try!(self.handle_get(i)),
            OpCode::BinGet(i) => try!(self.handle_get(i)),
            OpCode::LongBinGet(i) => try!(self.handle_get(i)),
            OpCode::Put(i) => try!(self.handle_put(i)),
            OpCode::BinPut(i) => try!(self.handle_put(i)),
            OpCode::LongBinPut(i) => try!(self.handle_put(i)),

            _ => return Err(Error::NotImplemented)
        }
        Ok(false)
    }
}