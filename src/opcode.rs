use std::io::{Read, BufRead, Error as IoError};
use std::str::{from_utf8, Utf8Error};

use num::bigint::{BigInt, ParseBigIntError};
use byteorder::{ReadBytesExt, LittleEndian, Error as ByteorderError};

quick_error! {
    #[derive(Debug)]
    pub enum Error {
        ReadError(err: ByteorderError) {
            from()
        }
        IoError(err: IoError) {
            from()
        }
        UnknownOpcode(opcode: u8) {}
        InvalidInt {
            from(Utf8Error)
            from(ParseBigIntError)
        }
        InvalidString
        ExpectedTrailingL
    }
}

#[derive(Debug)]
pub enum OpCode {
    Int(BigInt),
    BinInt(i32),
    BinInt1(u8),
    BinInt2(u16),
    Long(BigInt),
    Long1(u8),
    Long4(i32),

    String(Vec<u8>),
    BinString(Vec<u8>),
    ShortBinString(Vec<u8>),

    None,

    NewTrue,
    NewFalse,

    Unicode(String),
    BinUnicode(String),

    Float(f64),
    BinFloat(f64),

    EmptyList,
    Append,
    Appends,
    List,

    EmptyTuple,
    Tuple,
    Tuple1,
    Tuple2,
    Tuple3,

    EmptyDict,
    Dict,
    SetItem,
    SetItems,

    Pop,
    Dup,
    Mark,
    PopMark,

    Get(usize),
    BinGet(usize),
    LongBinGet(usize),
    Put(usize),
    BinPut(usize),
    LongBinPut(usize),

    Ext1(u8),
    Ext2(u16),
    Ext4(i32),

    Global(Vec<u8>, Vec<u8>),
    Reduce,
    Build,
    Inst(Vec<u8>, Vec<u8>),
    Obj,
    NewObj,
    Proto(u8),
    Stop,
    PersId(Vec<u8>),
    BinPersId,
}

pub fn read_opcode<R>(rd: &mut R) -> Result<OpCode, Error> where R: Read + BufRead {
    let marker = try!(rd.read_u8());
    return Ok(match marker {
        73 => {
            let mut buf = Vec::new();
            try!(rd.read_until('\n' as u8, &mut buf));

            // Skip last symbol — \n
            let init = match buf.split_last() {
                None => return Err(Error::InvalidString),
                Some((_last, init)) => init,
            };
            OpCode::Int(try!(try!(from_utf8(init)).parse()))
        },
        74 => OpCode::BinInt(try!(rd.read_i32::<LittleEndian>())),
        75 => OpCode::BinInt1(try!(rd.read_u8())),
        77 => OpCode::BinInt2(try!(rd.read_u16::<LittleEndian>())),
        76 => {

            let mut buf = Vec::new();
            try!(rd.read_until('\n' as u8, &mut buf));

            // Skip last symbol — \n
            let init_with_l = match buf.split_last() {
                None => return Err(Error::InvalidString),
                Some((_last, init)) => init,
            };

            let init = match init_with_l.split_last() {
                None => return Err(Error::InvalidString),
                Some((&76, init)) => init,
                Some(_) => return Err(Error::ExpectedTrailingL),
            };

            OpCode::Long(try!(try!(from_utf8(init)).parse()))
        },
        138 => OpCode::Long1(try!(rd.read_u8())),
        139 => OpCode::Long4(try!(rd.read_i32::<LittleEndian>())),
        c => return Err(Error::UnknownOpcode(c)),
    })
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor};

    use num::{FromPrimitive};

    use super::{OpCode, Error, read_opcode};

    macro_rules! t {
        ($buffer: expr, $pat:pat, $result:expr) => ({
            match read_opcode(&mut Cursor::new(&$buffer[..])) {
                $pat => $result,
                other => {
                    println!("ERROR {:?}", other);
                    assert!(false)
                },
            }
        })
    }

    #[test]
    fn test_int() {
        t!(b"I", Err(Error::InvalidString), assert!(true));
        t!(b"I\n", Err(Error::InvalidInt), assert!(true));
        t!(b"Iabc\n", Err(Error::InvalidInt), assert!(true));
        t!(b"I123\n", Ok(OpCode::Int(n)), assert_eq!(n, FromPrimitive::from_usize(123).unwrap()));
    }

    #[test]
    fn test_bin_int() {
        t!(b"J\x0a", Err(Error::ReadError(_)), assert!(true));
        t!(b"J\x0a\x00\x00\x00", Ok(OpCode::BinInt(n)), assert_eq!(n, 10));
        t!(b"J\x0a\x00\x00\x01", Ok(OpCode::BinInt(n)), assert_eq!(n, 16777226));
    }

    #[test]
    fn test_bin_int1() {
        t!(b"K", Err(Error::ReadError(_)), assert!(true));
        t!(b"K\x0a", Ok(OpCode::BinInt1(n)), assert_eq!(n, 10));
    }

    #[test]
    fn test_bin_int2() {
        t!(b"M\x0a", Err(Error::ReadError(_)), assert!(true));
        t!(b"M\x0a\x00\x00\x00", Ok(OpCode::BinInt2(n)), assert_eq!(n, 10));
        t!(b"M\x0a\x01\x00\x00", Ok(OpCode::BinInt2(n)), assert_eq!(n, 266));
    }

    #[test]
    fn test_long() {
        t!(b"L", Err(Error::InvalidString), assert!(true));
        t!(b"L\n", Err(Error::InvalidString), assert!(true));
        t!(b"Labc\n", Err(Error::ExpectedTrailingL), assert!(true));
        t!(b"LabcL\n", Err(Error::InvalidInt), assert!(true));
        t!(b"L123L\n", Ok(OpCode::Long(n)), assert_eq!(n, FromPrimitive::from_usize(123).unwrap()));
    }

    #[test]
    fn test_long1() {
        t!(b"\x8a", Err(Error::ReadError(_)), assert!(true));
        t!(b"\x8a\x0a", Ok(OpCode::Long1(n)), assert_eq!(n, 10));
    }

    #[test]
    fn test_long4() {
        t!(b"\x8b\x0a", Err(Error::ReadError(_)), assert!(true));
        t!(b"\x8b\x0a\x00\x00\x00", Ok(OpCode::Long4(n)), assert_eq!(n, 10));
        t!(b"\x8b\x0a\x00\x00\x01", Ok(OpCode::Long4(n)), assert_eq!(n, 16777226));
    }
}
