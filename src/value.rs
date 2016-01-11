use num::bigint::{BigInt};

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum Value {
    None,
    Bool(bool),
    SmallInt(usize),
    Int(BigInt),
    Float(f64),
    String(Vec<u8>),
    Unicode(String),
    List(Vec<Value>),
    Tuple(Vec<Value>),
    Dict(Vec<(Value, Value)>),
}
