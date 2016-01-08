use std::collections::{HashMap};

use num::bigint::{BigInt};

use opcode::{OpCode, BooleanOrInt};
use value::{Value};

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

    fn split_off(&mut self, at: usize) -> Vec<Value> {
        if at > self.stack.len() {
            panic!("Stack too small");
        }
        self.stack.split_off(at)
    }

    fn pop(&mut self) -> Value {
        match self.stack.pop() {
            None => panic!("Empty stack"),
            Some(value) => value,
        }
    }

    fn execute(&mut self, opcode: OpCode) {
        match opcode {
            OpCode::Proto(_) => (),
            OpCode::Stop => (),  // TODO: !!!

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
                let v = self.pop();
                match self.stack.last_mut() {
                    None => panic!("Empty stack"),
                    Some(&mut Value::List(ref mut list)) => list.push(v),
                    _ => panic!("Invalid value on stack"),
                }
            },
            OpCode::Appends => {
                match self.marker {
                    None => panic!("Empty marker"),
                    Some(marker) => {
                        let values = self.split_off(marker);
                        match self.stack.last_mut() {
                            None => panic!("Empty stack"),
                            Some(&mut Value::List(ref mut list)) => {
                                list.extend(values);
                            },
                            _ => panic!("Invalid value on stack"),
                        }
                    }
                }
            },
            OpCode::List => {
                match self.marker {
                    None => panic!("Empty marker"),
                    Some(marker) => {
                        let values = self.split_off(marker);
                        self.stack.push(Value::List(values));
                    }
                }
            },

            OpCode::EmptyTuple => self.stack.push(Value::Tuple(Vec::new())),
            OpCode::Tuple => {
                match self.marker {
                    None => panic!("Empty marker"),
                    Some(marker) => {
                        let values = self.split_off(marker);
                        self.stack.push(Value::Tuple(values));
                    }
                }
            },
            OpCode::Tuple1 => {
                let v1 = self.pop();
                self.stack.push(Value::Tuple(vec![v1]))
            },
            OpCode::Tuple2 => {
                let v1 = self.pop();
                let v2 = self.pop();
                self.stack.push(Value::Tuple(vec![v1, v2]))
            },
            OpCode::Tuple3 => {
                let v1 = self.pop();
                let v2 = self.pop();
                let v3 = self.pop();
                self.stack.push(Value::Tuple(vec![v1, v2, v3]))
            }

            OpCode::EmptyDict => self.stack.push(Value::Dict(Vec::new())),
            OpCode::Dict => {
                match self.marker {
                    None => panic!("Empty marker"),
                    Some(marker) => {
                        let mut values = self.split_off(marker);
                        let mut dict = Vec::new();

                        for i in 0 .. values.len() / 2 { // TODO: Check panic
                            let key = values.remove(2 * i);
                            let value = values.remove(2 * i + 1);
                            dict.push((key, value));
                        }
                        self.stack.push(Value::Dict(dict));
                    }
                }
            },
            OpCode::SetItem => {
                let value = self.pop();
                let key = self.pop();
                match self.stack.last_mut() {
                    None => panic!("Empty stack"),
                    Some(&mut Value::Dict(ref mut dict)) => dict.push((key, value)),
                    _ => panic!("Invalid value on stack"),
                }
            },
            OpCode::SetItems => {
                match self.marker {
                    None => panic!("Empty marker"),
                    Some(marker) => {
                        let mut values = self.split_off(marker);

                        match self.stack.last_mut() {
                            None => panic!("Empty stack"),
                            Some(&mut Value::Dict(ref mut dict)) => {
                                for i in 0 .. values.len() / 2 { // TODO: Check panic
                                    let key = values.remove(2 * i);
                                    let value = values.remove(2 * i + 1);
                                    dict.push((key, value));
                                }
                            },
                            _ => panic!("Invalid value on stack"),
                        }
                    }
                }
            },

            OpCode::Pop => {
                self.pop();
            },
            OpCode::Dup => {
                let value = match self.stack.last() {
                    None => panic!("Empty stack"),
                    Some(ref v) => (*v).clone(),
                };
                self.stack.push(value)
            },
            OpCode::Mark => {
                self.marker = Some(self.stack.len())
            },
            OpCode::PopMark => {
                match self.marker {
                    None => panic!("Empty marker"),
                    Some(marker) => {
                        self.split_off(marker);
                    }
                }
            },

            _ => panic!("Not implemented")
        }
    }
}