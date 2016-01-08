use num::bigint::{BigInt};

#[derive(Debug, PartialEq, PartialOrd)]
pub enum Value {
    None,
    Bool(bool),
    Int(BigInt),
    Float(f64),
    String(Vec<u8>),
    Unicode(String),
    List(Vec<Value>),
    Tuple(Vec<Value>),
    Dict(Vec<(Value, Value)>),
}
