use std::collections::{HashMap};

use num::bigint::{BigInt};

use opcode::{OpCode, BooleanOrInt};
use value::{Value};

pub struct Machine {
    stack: Vec<Value>,
    memo: HashMap<usize, Value>,
}

impl Machine {
    fn new() -> Self {
        Machine {
            stack: Vec::new(),
            memo: HashMap::new(),
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
            _ => panic!("Not implemented")
        }
    }
}