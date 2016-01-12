// Copyright (c) 2016 Fedor Gogolev <knsd@knsd.net>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use num::bigint::{BigInt};

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum Value {
    None,
    Bool(bool),
    Int(usize),
    Long(BigInt),
    Float(f64),
    String(Vec<u8>),
    Unicode(String),
    List(Vec<Value>),
    Tuple(Vec<Value>),
    Dict(Vec<(Value, Value)>),
}
