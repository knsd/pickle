// Copyright (c) 2016 Fedor Gogolev <knsd@knsd.net>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::cell::{RefCell};
use std::rc::{Rc};

use num::bigint::{BigInt};

#[derive(Debug, Clone)]
pub enum Value {
    None,
    Bool(bool),
    Int(isize),
    Long(BigInt),
    Float(f64),
    String(Vec<u8>),
    Unicode(String),
    List(Rc<RefCell<Vec<Value>>>),
    Tuple(Rc<RefCell<Vec<Value>>>),
    Dict(Rc<RefCell<Vec<(Value, Value)>>>),
}
