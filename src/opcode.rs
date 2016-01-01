use std::io::{Read, BufRead, Error as IoError, ErrorKind};
use std::str::{from_utf8, Utf8Error};
use std::string::{FromUtf8Error};
use std::num::{ParseIntError, ParseFloatError};

use num::{Zero};
use num::bigint::{BigInt, ToBigInt, Sign, ParseBigIntError};
use byteorder::{ReadBytesExt, LittleEndian, BigEndian, Error as ByteorderError};

use string::{unescape, Error as UnescapeError};

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
            from(ParseIntError)
            from(ParseBigIntError)
        }
        InvalidFloat {
            from(Utf8Error)
            from(ParseFloatError)
        }
        InvalidString {
            from(FromUtf8Error)
        }
        InvalidProto
        ExpectedTrailingL
        InvalidLong
        NegativeLength
        UnescapeError(err: UnescapeError) {
            from()
        }
    }
}

#[derive(Debug)]
pub enum OpCode {
    Proto(u8),
    Stop,

    Int(i64),  // TODO: Boolean can be encoded here
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

    // TODO: why usize
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
        Some((&b'\n', init)) => Ok(init.to_vec()),
        _ => return Err(Error::InvalidString),
    }
}

pub fn dec_to_digit(c: u8) -> Option<u8> {
    let val = match c {
        b'0' ... b'9' => c - b'0',
        _ => return None,
    };
    Some(val)
}

fn from_bytes(src: &[u8]) -> Result<i64, Error> {
    if src.is_empty() {
        return Err(Error::InvalidInt);
    }

    let (is_positive, digits) = match src[0] {
        b'+' => (true, &src[1..]),
        b'-' => (false, &src[1..]),
        _ => (true, src)
    };

    if digits.is_empty() {
        return Err(Error::InvalidInt);
    }

    let mut result = 0;

    if is_positive {
        // The number is positive
        for &c in digits {
            let x = match dec_to_digit(c) {
                Some(x) => x as i64,
                None => return Err(Error::InvalidInt),
            };
            result = result * 10;
            result = result + x;
        }
    } else {
        // The number is negative
        for &c in digits {
            let x = match dec_to_digit(c) {
                Some(x) => x as i64,
                None => return Err(Error::InvalidInt),
            };
            result = result * 10;
            result = result - x;
        }
    }
    Ok(result)
}

fn read_decimal_int<R>(rd: &mut R) -> Result<i64, Error> where R: Read + BufRead {
    let s = try!(read_until_newline(rd));
    Ok(try!(from_bytes(&s)))
}

fn read_decimal_long<R>(rd: &mut R) -> Result<BigInt, Error> where R: Read + BufRead {
    let s = try!(read_until_newline(rd));
    let init = match s.split_last() {
        None => return Err(Error::InvalidString),
        Some((&b'L', init)) => init,
        Some(_) => return Err(Error::ExpectedTrailingL),
    };

    match BigInt::parse_bytes(&init, 10) {
        Some(i) => Ok(i),
        None => Err(Error::InvalidInt)
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

    macro_rules! ensure_not_negative {
        ($n: expr) => ({
            if $n < Zero::zero() {
                return Err(Error::NegativeLength)
            }
        })
    }

    let marker = try!(rd.read_u8());
    return Ok(match marker {
        b'\x80' => {
            let version = try!(rd.read_u8());
            if version < 2 {
                return Err(Error::InvalidProto)
            }
            OpCode::Proto(version)
        }
        b'.' => OpCode::Stop,
        b'I' => OpCode::Int(try!(read_decimal_int(rd))),
        b'J' => OpCode::BinInt(try!(rd.read_i32::<LittleEndian>())),
        b'K' => OpCode::BinInt1(try!(rd.read_u8())),
        b'M' => OpCode::BinInt2(try!(rd.read_u16::<LittleEndian>())),
        b'L' => OpCode::Long(try!(read_decimal_long(rd))),
        b'\x8a' => {
            let length = try!(rd.read_u8());
            OpCode::Long1(try!(read_long(rd, length as usize)))
        },
        b'\x8b' => {
            let length = try!(rd.read_i32::<LittleEndian>());
            ensure_not_negative!(length);

            OpCode::Long4(try!(read_long(rd, length as usize)))
        },

        b'S' => OpCode::String(try!(unescape(&try!(read_until_newline(rd))))),
        b'T' => {
            let length = try!(rd.read_i32::<LittleEndian>());
            ensure_not_negative!(length);

            let mut buf = vec![0; length as usize];
            try!(read_exact(rd, &mut buf));
            OpCode::BinString(buf)
        }
        b'U' => {
            let length = try!(rd.read_u8());
            let mut buf = vec![0; length as usize];
            try!(read_exact(rd, &mut buf));
            OpCode::ShortBinString(buf)
        }

        b'N' => OpCode::None,
        b'\x88' => OpCode::NewTrue,
        b'\x89' => OpCode::NewFalse,

        b'V' => unimplemented!(), // Unicode
        b'X' => {
            let length = try!(rd.read_i32::<LittleEndian>());
            ensure_not_negative!(length);
            let mut buf = vec![0; length as usize];
            try!(read_exact(rd, buf.as_mut()));
            OpCode::BinUnicode(try!(String::from_utf8(buf)))
        },

        b'F' => {
            let s = try!(read_until_newline(rd));
            OpCode::Float(try!(try!(from_utf8(&s)).parse()))
        },
        b'G' => {
            OpCode::BinFloat(try!(rd.read_f64::<BigEndian>()))
        }

        b']' => OpCode::EmptyList,
        b'a' => OpCode::Append,
        b'e' => OpCode::Appends,
        b'l' => OpCode::List,

        b')' => OpCode::EmptyTuple,
        b't' => OpCode::Tuple,
        b'\x85' => OpCode::Tuple1,
        b'\x86' => OpCode::Tuple2,
        b'\x87' => OpCode::Tuple3,

        b'}' => OpCode::EmptyDict,
        b'd' => OpCode::Dict,
        b's' => OpCode::SetItem,
        b'u' => OpCode::SetItems,

        b'0' => OpCode::Pop,
        b'2' => OpCode::Dup,
        b'(' => OpCode::Mark,
        b'1' => OpCode::PopMark,

        b'g' => {
            let n = try!(read_decimal_int(rd));
            ensure_not_negative!(n);
            OpCode::Get(n as usize)
        },
        b'h' => OpCode::BinGet(try!(rd.read_u8()) as usize),
        b'j' => {
            let n = try!(rd.read_i32::<LittleEndian>());
            ensure_not_negative!(n);
            OpCode::LongBinGet(n as usize)
        }
        b'p' => {
            let n = try!(read_decimal_int(rd));
            ensure_not_negative!(n);
            OpCode::Put(n as usize)
        },
        b'q' => OpCode::BinPut(try!(rd.read_u8()) as usize),
        b'r' => {
            let n = try!(rd.read_i32::<LittleEndian>());
            ensure_not_negative!(n);
            OpCode::LongBinPut(n as usize)
        }

        b'\x82' => OpCode::Ext1(try!(rd.read_u8())),
        b'\x83' => OpCode::Ext2(try!(rd.read_u16::<LittleEndian>())),
        b'\x84' => OpCode::Ext4(try!(rd.read_i32::<LittleEndian>())),  // TODO: ensure_not_negative?

        b'c' => OpCode::Global(try!(read_until_newline(rd)), try!(read_until_newline(rd))),
        b'R' => OpCode::Reduce,
        b'b' => OpCode::Build,
        b'i' => OpCode::Inst(try!(read_until_newline(rd)), try!(read_until_newline(rd))),
        b'o' => OpCode::Obj,
        b'\x81' => OpCode::NewObj,
        b'P' => OpCode::PersId(try!(read_until_newline(rd))),
        b'Q' => OpCode::BinPersId,

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
                Ok($pat) => $result,
                other => {
                    println!("ERROR {:?}", other);
                    assert!(false)
                },
            }
        })
    }

    macro_rules! e {
        ($buffer: expr, $pat:pat) => ({
            match read_opcode(&mut Cursor::new(&$buffer[..])) {
                Err($pat) => (),
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
    fn test_proto() {
        e!(b"\x80", Error::ReadError(_));
        e!(b"\x80\x00", Error::InvalidProto);
        e!(b"\x80\x01", Error::InvalidProto);
        t!(b"\x80\x02", OpCode::Proto(n), assert_eq!(n, 2));
        t!(b"\x80\x0a", OpCode::Proto(n), assert_eq!(n, 10));
    }

    fn test_stop() {
        t!(b".", OpCode::Stop, ());
    }

    #[test]
    fn test_int() {
        e!(b"I", Error::InvalidString);
        e!(b"I\n", Error::InvalidInt);
        e!(b"Iabc\n", Error::InvalidInt);
        t!(b"I123\n", OpCode::Int(n), assert_eq!(n, 123));
        t!(b"I-123\n", OpCode::Int(n), assert_eq!(n, -123));
    }

    #[test]
    fn test_bin_int() {
        e!(b"J\x0a", Error::ReadError(_));
        t!(b"J\x0a\x00\x00\x00", OpCode::BinInt(n), assert_eq!(n, 10));
        t!(b"J\x0a\x00\x00\x01", OpCode::BinInt(n), assert_eq!(n, 16777226));
    }

    #[test]
    fn test_bin_int1() {
        e!(b"K", Error::ReadError(_));
        t!(b"K\x0a", OpCode::BinInt1(n), assert_eq!(n, 10));
    }

    #[test]
    fn test_bin_int2() {
        e!(b"M\x0a", Error::ReadError(_));
        t!(b"M\x0a\x00\x00\x00", OpCode::BinInt2(n), assert_eq!(n, 10));
        t!(b"M\x0a\x01\x00\x00", OpCode::BinInt2(n), assert_eq!(n, 266));
    }

    #[test]
    fn test_long() {
        e!(b"L", Error::InvalidString);
        e!(b"L\n", Error::InvalidString);
        e!(b"Labc\n", Error::ExpectedTrailingL);
        e!(b"LabcL\n", Error::InvalidInt);
        t!(b"L123L\n", OpCode::Long(n), assert_eq!(n, n!(123)));
    }

    #[test]
    fn test_long1() {
        e!(b"\x8a", Error::ReadError(_));
        t!(b"\x8a\x01\x0a", OpCode::Long1(n), assert_eq!(n, n!(10)));
        t!(b"\x8a\x01\xf6", OpCode::Long1(n), assert_eq!(n, n!(-10)));
        t!(b"\x8a\x02.\xfb", OpCode::Long1(n), assert_eq!(n, n!(-1234)));
    }

    #[test]
    fn test_long4() {
        e!(b"\x8b\xff\xff\xff\xff", Error::NegativeLength);
        e!(b"\x8b\x0a", Error::ReadError(_));
        t!(b"\x8b\x01\x00\x00\x00\x0a", OpCode::Long4(n), assert_eq!(n, n!(10)));
        t!(b"\x8b\x01\x00\x00\x00\xf6", OpCode::Long4(n), assert_eq!(n, n!(-10)));
        t!(b"\x8b\x02\x00\x00\x00.\xfb", OpCode::Long4(n), assert_eq!(n, n!(-1234)));
    }

    #[test]
    fn test_string() {
        e!(b"S", Error::InvalidString);
        t!(b"S\n", OpCode::String(s), assert_eq!(s, b""));
        t!(b"Sabc\n", OpCode::String(s), assert_eq!(s, b"abc"));
        t!(b"S123\n", OpCode::String(s), assert_eq!(s, b"123"));
        t!(b"S\\n\n", OpCode::String(s), assert_eq!(s, b"\n"));
    }

    #[test]
    fn test_bin_string() {
        e!(b"T\xff\xff\xff\xff", Error::NegativeLength);
        t!(b"T\x00\x00\x00\x00", OpCode::BinString(s), assert_eq!(s, b""));
        t!(b"T\x03\x00\x00\x00abc", OpCode::BinString(s), assert_eq!(s, b"abc"));
        t!(b"T\x03\x00\x00\x00123", OpCode::BinString(s), assert_eq!(s, b"123"));
        t!(b"T\x02\x00\x00\x00\\n", OpCode::BinString(s), assert_eq!(s, b"\\n"));
    }

    #[test]
    fn test_short_bin_string() {
        t!(b"U\x00", OpCode::ShortBinString(s), assert_eq!(s, b""));
        t!(b"U\x03abc", OpCode::ShortBinString(s), assert_eq!(s, b"abc"));
        t!(b"U\x03123", OpCode::ShortBinString(s), assert_eq!(s, b"123"));
        t!(b"U\x02\\n", OpCode::ShortBinString(s), assert_eq!(s, b"\\n"));
    }

    #[test]
    fn test_none() {
        t!(b"N", OpCode::None, ());
    }

    #[test]
    fn test_new_true() {
        t!(b"\x88", OpCode::NewTrue, ());
    }

    #[test]
    fn test_new_false() {
        t!(b"\x89", OpCode::NewFalse, ());
    }

    #[test]
    fn test_unicode() {
    }

    #[test]
    fn test_bin_unicode() {
        e!(b"X\t\x00\x00\x00abc\xd0\xb3\xb4\xd0\xb5q", Error::InvalidString);
        t!(b"X\t\x00\x00\x00abc\xd0\xb3\xd0\xb4\xd0\xb5q", OpCode::BinUnicode(s), assert_eq!(s, "abcгде"));
    }

    #[test]
    fn test_float() {
        e!(b"F", Error::InvalidString);
        e!(b"F\n", Error::InvalidFloat);
        e!(b"Fabc\n", Error::InvalidFloat);
        t!(b"F123\n", OpCode::Float(n), assert_eq!(n, 123.0));
        t!(b"F-123\n", OpCode::Float(n), assert_eq!(n, -123.0));
        t!(b"F-123.\n", OpCode::Float(n), assert_eq!(n, -123.0));
        t!(b"F-123.456\n", OpCode::Float(n), assert_eq!(n, -123.456));
    }

    #[test]
    fn test_bin_float() {
        e!(b"G", Error::ReadError(_));
        e!(b"Gabc", Error::ReadError(_));
        e!(b"G123", Error::ReadError(_));
        t!(b"G@^\xc0\x00\x00\x00\x00\x00", OpCode::BinFloat(n), assert_eq!(n, 123.0));
        t!(b"G\xc0^\xc0\x00\x00\x00\x00\x00", OpCode::BinFloat(n), assert_eq!(n, -123.0));
        t!(b"G\xc0^\xdd/\x1a\x9f\xbew", OpCode::BinFloat(n), assert_eq!(n, -123.456));
    }

    #[test]
    fn test_empty_list() {
        t!(b"]", OpCode::EmptyList, ());
    }

    #[test]
    fn test_append() {
        t!(b"a", OpCode::Append, ());
    }

    #[test]
    fn test_appends() {
        t!(b"e", OpCode::Appends, ());
    }

    #[test]
    fn test_list() {
        t!(b"l", OpCode::List, ());
    }

    #[test]
    fn test_empty_tuple() {
        t!(b")", OpCode::EmptyTuple, ());
    }

    #[test]
    fn test_tuple() {
        t!(b"t", OpCode::Tuple, ());
    }

    #[test]
    fn test_tuple1() {
        t!(b"\x85", OpCode::Tuple1, ());
    }

    #[test]
    fn test_tuple2() {
        t!(b"\x86", OpCode::Tuple2, ());
    }

    #[test]
    fn test_tuple3() {
        t!(b"\x87", OpCode::Tuple3, ());
    }

    #[test]
    fn test_empty_dict() {
        t!(b"}", OpCode::EmptyDict, ());
    }

    #[test]
    fn test_dict() {
        t!(b"d", OpCode::Dict, ());
    }

    #[test]
    fn test_set_item() {
        t!(b"s", OpCode::SetItem, ());
    }

    #[test]
    fn test_set_items() {
        t!(b"u", OpCode::SetItems, ());
    }

    #[test]
    fn test_pop() {
        t!(b"0", OpCode::Pop, ());
    }

    #[test]
    fn test_dup() {
        t!(b"2", OpCode::Dup, ());
    }

    #[test]
    fn test_mark() {
        t!(b"(", OpCode::Mark, ());
    }

    #[test]
    fn test_pop_mark() {
        t!(b"1", OpCode::PopMark, ());
    }

    #[test]
    fn test_get() {
        e!(b"g", Error::InvalidString);
        e!(b"g\n", Error::InvalidInt);
        e!(b"gabc\n", Error::InvalidInt);
        e!(b"g-123\n", Error::NegativeLength);
        t!(b"g123\n", OpCode::Get(n), assert_eq!(n, 123));
    }

    #[test]
    fn test_bin_get() {
        e!(b"h", Error::ReadError(_));
        t!(b"h\x00", OpCode::BinGet(n), assert_eq!(n, 0));
        t!(b"h\x0a", OpCode::BinGet(n), assert_eq!(n, 10));
        t!(b"h\xfe", OpCode::BinGet(n), assert_eq!(n, 254));

    }

    #[test]
    fn test_long_bin_get() {
        e!(b"j\x0a", Error::ReadError(_));
        t!(b"j\x0a\x00\x00\x00", OpCode::LongBinGet(n), assert_eq!(n, 10));
        t!(b"j\x0a\x00\x00\x01", OpCode::LongBinGet(n), assert_eq!(n, 16777226));
    }

    #[test]
    fn test_put() {
        e!(b"p", Error::InvalidString);
        e!(b"p\n", Error::InvalidInt);
        e!(b"pabc\n", Error::InvalidInt);
        e!(b"p-123\n", Error::NegativeLength);
        t!(b"p123\n", OpCode::Put(n), assert_eq!(n, 123));
    }

    #[test]
    fn test_bin_put() {
        e!(b"q", Error::ReadError(_));
        t!(b"q\x00", OpCode::BinPut(n), assert_eq!(n, 0));
        t!(b"q\x0a", OpCode::BinPut(n), assert_eq!(n, 10));
        t!(b"q\xfe", OpCode::BinPut(n), assert_eq!(n, 254));
    }

    #[test]
    fn test_long_bin_put() {
        e!(b"r\x0a", Error::ReadError(_));
        t!(b"r\x0a\x00\x00\x00", OpCode::LongBinPut(n), assert_eq!(n, 10));
        t!(b"r\x0a\x00\x00\x01", OpCode::LongBinPut(n), assert_eq!(n, 16777226));
    }

    #[test]
    fn test_ext1() {
        e!(b"\x82", Error::ReadError(_));
        t!(b"\x82\x0a", OpCode::Ext1(n), assert_eq!(n, 10));
    }

    #[test]
    fn test_ext2() {
        e!(b"\x83", Error::ReadError(_));
        e!(b"\x83\x01", Error::ReadError(_));
        t!(b"\x83\x0a\x00", OpCode::Ext2(n), assert_eq!(n, 10));
        t!(b"\x83\x0a\x01", OpCode::Ext2(n), assert_eq!(n, 266));
    }

    #[test]
    fn test_ext4() {
        e!(b"\x84", Error::ReadError(_));
        e!(b"\x84\x01\x01\x01", Error::ReadError(_));
        t!(b"\x84\x0a\x00\x00\x00", OpCode::Ext4(n), assert_eq!(n, 10));
        t!(b"\x84\x0a\x01\x00\x01", OpCode::Ext4(n), assert_eq!(n, 16777482));
    }

    #[test]
    fn test_global() {
        e!(b"c", Error::InvalidString);
        e!(b"c\n", Error::InvalidString);
        t!(b"c\n\n", OpCode::Global(a, b), {assert_eq!(a, b""); assert_eq!(b, b"");});
        t!(b"cmodule\nclass\n", OpCode::Global(a, b), {assert_eq!(a, b"module"); assert_eq!(b, b"class");});
    }

    #[test]
    fn test_reduce() {
        t!("R", OpCode::Reduce, ())
    }

    #[test]
    fn test_build() {
        t!("b", OpCode::Build, ())
    }

    #[test]
    fn test_inst() {
        e!(b"i", Error::InvalidString);
        e!(b"i\n", Error::InvalidString);
        t!(b"i\n\n", OpCode::Inst(a, b), {assert_eq!(a, b""); assert_eq!(b, b"");});
        t!(b"imodule\nclass\n", OpCode::Inst(a, b), {assert_eq!(a, b"module"); assert_eq!(b, b"class");});
    }

    #[test]
    fn test_obj() {
        t!("o", OpCode::Obj, ())
    }

    #[test]
    fn test_new_obj() {
        t!(b"\x81", OpCode::NewObj, ())
    }

    #[test]
    fn test_persid() {
        e!(b"P", Error::InvalidString);
        e!(b"Pabc", Error::InvalidString);
        t!(b"P\n", OpCode::PersId(a), assert_eq!(a, b""));
        t!(b"P\n\n", OpCode::PersId(a), assert_eq!(a, b""));
        t!(b"Pmodule\nclass\n", OpCode::PersId(a), assert_eq!(a, b"module"));
    }

    #[test]
    fn test_bin_persid() {
        t!(b"Q", OpCode::BinPersId, ())
    }
}
