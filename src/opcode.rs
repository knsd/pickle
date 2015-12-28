use std::io::{Read, BufRead, Error as IoError, ErrorKind};
use std::str::{from_utf8, Utf8Error};
use std::num::{ParseFloatError};

use num::bigint::{BigInt, ToBigInt, Sign, ParseBigIntError};
use byteorder::{ReadBytesExt, LittleEndian, BigEndian, Error as ByteorderError};

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
        InvalidFloat {
            from(ParseFloatError)
        }
        InvalidString
        ExpectedTrailingL
        InvalidLong
        NegativeLength
    }
}

#[derive(Debug)]
pub enum OpCode {
    Int(BigInt),
    BinInt(i32),
    BinInt1(u8),
    BinInt2(u16),
    Long(BigInt),
    Long1(BigInt),
    Long4(BigInt),

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
        None => Err(Error::InvalidString),
        Some((_last, init)) => Ok(init.to_vec()),
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

pub fn read_opcode<R>(rd: &mut R) -> Result<OpCode, Error> where R: Read + BufRead {
    let marker = try!(rd.read_u8());
    return Ok(match marker {
        73 => {
            let s = try!(read_until_newline(rd));
            OpCode::Int(try!(try!(from_utf8(&s)).parse()))  // http://rust-num.github.io/num/num/bigint/struct.BigInt.html#method.parse_bytes
        },
        74 => OpCode::BinInt(try!(rd.read_i32::<LittleEndian>())),
        75 => OpCode::BinInt1(try!(rd.read_u8())),
        77 => OpCode::BinInt2(try!(rd.read_u16::<LittleEndian>())),
        76 => {
            let s = try!(read_until_newline(rd));

            let init = match s.split_last() {
                None => return Err(Error::InvalidString),
                Some((&76, init)) => init,
                Some(_) => return Err(Error::ExpectedTrailingL),
            };

            OpCode::Long(try!(try!(from_utf8(init)).parse()))
        },
        138 => {
            let length = try!(rd.read_u8());
            OpCode::Long1(try!(read_long(rd, length as usize)))

        },
        139 => {
            let length = try!(rd.read_i32::<LittleEndian>());
            if length < 0 {
                return Err(Error::NegativeLength)
            }
            OpCode::Long4(try!(read_long(rd, length as usize)))

        },

        83 => {OpCode::String(try!(read_until_newline(rd)))} // TODO: escaping
        84 => {
            let length = try!(rd.read_i32::<LittleEndian>());
            if length < 0 {
                return Err(Error::NegativeLength)
            }
            let mut buf = vec![0; length as usize];
            try!(read_exact(rd, &mut buf));
            OpCode::BinString(buf)
        }
        85 => {
            let length = try!(rd.read_u8());
            let mut buf = vec![0; length as usize];
            try!(read_exact(rd, &mut buf));
            OpCode::ShortBinString(buf)
        }

        78 => OpCode::None,
        136 => OpCode::NewTrue,
        137 => OpCode::NewFalse,

        86 => unimplemented!(), // Unicode
        88 => unimplemented!(), // BinUnicode

        70 => {
            let s = try!(read_until_newline(rd));
            OpCode::Float(try!(try!(from_utf8(&s)).parse()))
        },
        71 => {
            OpCode::BinFloat(try!(rd.read_f64::<BigEndian>()))
        }

        93 => OpCode::EmptyList,
        97 => OpCode::Append,
        101 => OpCode::Appends,
        108 => OpCode::List,

        41 => OpCode::EmptyTuple,
        116 => OpCode::Tuple,
        133 => OpCode::Tuple1,
        134 => OpCode::Tuple2,
        135 => OpCode::Tuple3,

        125 => OpCode::EmptyDict,
        100 => OpCode::Dict,
        115 => OpCode::SetItem,
        117 => OpCode::SetItems,

        48 => OpCode::Pop,
        50 => OpCode::Dup,
        40 => OpCode::Mark,
        49 => OpCode::PopMark,

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

    macro_rules! n {
        ($x: expr) => ({FromPrimitive::from_isize($x).unwrap()})
    }

    #[test]
    fn test_int() {
        t!(b"I", Err(Error::InvalidString), assert!(true));
        t!(b"I\n", Err(Error::InvalidInt), assert!(true));
        t!(b"Iabc\n", Err(Error::InvalidInt), assert!(true));
        t!(b"I123\n", Ok(OpCode::Int(n)), assert_eq!(n, n!(123)));
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
        t!(b"L123L\n", Ok(OpCode::Long(n)), assert_eq!(n, n!(123)));
    }

    #[test]
    fn test_long1() {
        t!(b"\x8a", Err(Error::ReadError(_)), assert!(true));
        t!(b"\x8a\x01\x0a", Ok(OpCode::Long1(n)), assert_eq!(n, n!(10)));
        t!(b"\x8a\x01\xf6", Ok(OpCode::Long1(n)), assert_eq!(n, n!(-10)));
        t!(b"\x8a\x02.\xfb", Ok(OpCode::Long1(n)), assert_eq!(n, n!(-1234)));
    }

    #[test]
    fn test_long4() {
        t!(b"\x8b\xff\xff\xff\xff", Err(Error::NegativeLength), assert!(true));
        t!(b"\x8b\x0a", Err(Error::ReadError(_)), assert!(true));
        t!(b"\x8b\x01\x00\x00\x00\x0a", Ok(OpCode::Long4(n)), assert_eq!(n, n!(10)));
        t!(b"\x8b\x01\x00\x00\x00\xf6", Ok(OpCode::Long4(n)), assert_eq!(n, n!(-10)));
        t!(b"\x8b\x02\x00\x00\x00.\xfb", Ok(OpCode::Long4(n)), assert_eq!(n, n!(-1234)));
    }

    #[test]
    fn test_string() {
        t!(b"S", Err(Error::InvalidString), assert!(true));
        t!(b"S\n", Ok(OpCode::String(s)), assert_eq!(s, b""));
        t!(b"Sabc\n", Ok(OpCode::String(s)), assert_eq!(s, b"abc"));
        t!(b"S123\n", Ok(OpCode::String(s)), assert_eq!(s, b"123"));
        t!(b"S\\n\n", Ok(OpCode::String(s)), assert_eq!(s, b"\\n"));
    }

    #[test]
    fn test_bin_string() {
        t!(b"T\xff\xff\xff\xff", Err(Error::NegativeLength), assert!(true));
        t!(b"T\x00\x00\x00\x00", Ok(OpCode::BinString(s)), assert_eq!(s, b""));
        t!(b"T\x03\x00\x00\x00abc", Ok(OpCode::BinString(s)), assert_eq!(s, b"abc"));
        t!(b"T\x03\x00\x00\x00123", Ok(OpCode::BinString(s)), assert_eq!(s, b"123"));
        t!(b"T\x02\x00\x00\x00\\n", Ok(OpCode::BinString(s)), assert_eq!(s, b"\\n"));
    }

    #[test]
    fn test_short_bin_string() {
        t!(b"U\x00", Ok(OpCode::ShortBinString(s)), assert_eq!(s, b""));
        t!(b"U\x03abc", Ok(OpCode::ShortBinString(s)), assert_eq!(s, b"abc"));
        t!(b"U\x03123", Ok(OpCode::ShortBinString(s)), assert_eq!(s, b"123"));
        t!(b"U\x02\\n", Ok(OpCode::ShortBinString(s)), assert_eq!(s, b"\\n"));
    }

    #[test]
    fn test_none() {
        t!(b"N", Ok(OpCode::None), assert!(true));
    }

    #[test]
    fn test_new_true() {
        t!(b"\x88", Ok(OpCode::NewTrue), assert!(true));
    }

    #[test]
    fn test_new_false() {
        t!(b"\x89", Ok(OpCode::NewFalse), assert!(true));
    }

    #[test]
    fn test_unicode() {
    }

    #[test]
    fn test_bin_unicode() {
    }

    #[test]
    fn test_float() {
        t!(b"F", Err(Error::InvalidString), assert!(true));
        t!(b"F\n", Err(Error::InvalidFloat), assert!(true));
        t!(b"Fabc\n", Err(Error::InvalidFloat), assert!(true));
        t!(b"F123\n", Ok(OpCode::Float(n)), assert_eq!(n, 123.0));
        t!(b"F-123\n", Ok(OpCode::Float(n)), assert_eq!(n, -123.0));
        t!(b"F-123.\n", Ok(OpCode::Float(n)), assert_eq!(n, -123.0));
        t!(b"F-123.456\n", Ok(OpCode::Float(n)), assert_eq!(n, -123.456));
    }

    #[test]
    fn test_bin_float() {
        t!(b"G", Err(Error::ReadError(_)), assert!(true));
        t!(b"Gabc", Err(Error::ReadError(_)), assert!(true));
        t!(b"G123", Err(Error::ReadError(_)), assert!(true));
        t!(b"G@^\xc0\x00\x00\x00\x00\x00", Ok(OpCode::BinFloat(n)), assert_eq!(n, 123.0));
        t!(b"G\xc0^\xc0\x00\x00\x00\x00\x00", Ok(OpCode::BinFloat(n)), assert_eq!(n, -123.0));
        t!(b"G\xc0^\xdd/\x1a\x9f\xbew", Ok(OpCode::BinFloat(n)), assert_eq!(n, -123.456));
    }

    #[test]
    fn test_empty_list() {
        t!(b"]", Ok(OpCode::EmptyList), assert!(true));
    }

    #[test]
    fn test_append() {
        t!(b"a", Ok(OpCode::Append), assert!(true));
    }

    #[test]
    fn test_appends() {
        t!(b"e", Ok(OpCode::Appends), assert!(true));
    }

    #[test]
    fn test_list() {
        t!(b"l", Ok(OpCode::List), assert!(true));
    }

    #[test]
    fn test_empty_tuple() {
        t!(b")", Ok(OpCode::EmptyTuple), assert!(true));
    }

    #[test]
    fn test_tuple() {
        t!(b"t", Ok(OpCode::Tuple), assert!(true));
    }

    #[test]
    fn test_tuple1() {
        t!(b"\x85", Ok(OpCode::Tuple1), assert!(true));
    }

    #[test]
    fn test_tuple2() {
        t!(b"\x86", Ok(OpCode::Tuple2), assert!(true));
    }

    #[test]
    fn test_tuple3() {
        t!(b"\x87", Ok(OpCode::Tuple3), assert!(true));
    }

    #[test]
    fn test_empty_dict() {
        t!(b"}", Ok(OpCode::EmptyDict), assert!(true));
    }

    #[test]
    fn test_dict() {
        t!(b"d", Ok(OpCode::Dict), assert!(true));
    }

    #[test]
    fn test_set_item() {
        t!(b"s", Ok(OpCode::SetItem), assert!(true));
    }

    #[test]
    fn test_set_items() {
        t!(b"u", Ok(OpCode::SetItems), assert!(true));
    }

    #[test]
    fn test_pop() {
        t!(b"0", Ok(OpCode::Pop), assert!(true));
    }

    #[test]
    fn test_dup() {
        t!(b"2", Ok(OpCode::Dup), assert!(true));
    }

    #[test]
    fn test_mark() {
        t!(b"(", Ok(OpCode::Mark), assert!(true));
    }

    #[test]
    fn test_pop_mark() {
        t!(b"1", Ok(OpCode::PopMark), assert!(true));
    }


}
