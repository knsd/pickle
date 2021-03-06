// Copyright (c) 2016 Fedor Gogolev <knsd@knsd.net>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::collections::{VecDeque};
use std::char::{from_u32};

use unicode_names::{character};
use from_ascii::{FromAsciiRadix, ParseIntError};

quick_error! {
    #[derive(Debug)]
    pub enum Error {
        InvalidValue {
            from(ParseIntError)
        }
        UnexpectedEnd
    }
}

pub fn unescape(s: &[u8], unicode: bool) -> Result<Vec<u8>, Error> {
    let mut buf = Vec::with_capacity(s.len());
    let mut oct_buf = VecDeque::with_capacity(3);

    let mut i = 0;

    macro_rules! read {
        () => ({
            match s.get(i) {
                None => return Err(Error::UnexpectedEnd),
                Some(c) => {
                    i += 1;
                    *c
                }
            }
        })
    }

    macro_rules! peek {
        () => ({
            s.get(i).cloned()
        })
    }

    macro_rules! push_char {
        ($c: ident) => ({
            let mut s = String::new();
            s.push($c);
            buf.extend_from_slice(s.as_bytes());
        })
    }

    loop {
        if i >= s.len() {
            return Ok(buf)
        }

        let c = read!();

        if c != b'\\' {
            buf.push(c);
            continue
        }

        let marker = read!();

        match marker {
            b'\n' => (),
            b'\\' => buf.push(b'\\'),
            b'\'' => buf.push(b'\''),
            b'"' => buf.push(b'"'),
            b'a' => buf.push(b'\x07'),
            b'b' => buf.push(b'\x08'),
            b'f' => buf.push(b'\x0c'),
            b'n' => buf.push(b'\n'),
            b'r' => buf.push(b'\r'),
            b't' => buf.push(b'\t'),
            b'v' => buf.push(b'\x0b'),
            b'x' => {
                let hex_buf = [read!(), read!()];
                buf.push(try!(u8::from_ascii_radix(&hex_buf, 16)))
            }
            b'0' ... b'7' => {
                oct_buf.push_back(marker);
                peek!().map(|c| {
                    if c >= b'0' && c <= b'7' {
                        oct_buf.push_back(c);
                        i += 1;

                        peek!().map(|c| {
                            if c >= b'0' && c <= b'7' {
                                oct_buf.push_back(c);
                                i += 1;
                            }
                        });
                    }
                });

                let value = try!(u16::from_ascii_radix(oct_buf.as_slices().0, 8));
                oct_buf.clear();
                buf.push(if value > 255 {
                    255
                } else {
                    value as u8
                });
                continue
            },
            b'u' if unicode => {
                let hex_buf = [read!(), read!(), read!(), read!()];
                let value = try!(u16::from_ascii_radix(&hex_buf, 16));
                let s = match String::from_utf16(&[value]) {
                    Ok(s) => s,
                    Err(_) => return Err(Error::InvalidValue),
                };
                buf.extend_from_slice(s.as_bytes());
            },
            b'U' if unicode => {
                let hex_buf = [read!(), read!(), read!(), read!(), read!(), read!(), read!(), read!()];
                let value = try!(u32::from_ascii_radix(&hex_buf, 16));
                match from_u32(value) {
                    Some(character) => push_char!(character),
                    None => return Err(Error::InvalidValue),
                };
            },
            b'N' if unicode => {
                if read!() != b'{' {
                    return Err(Error::InvalidValue)
                }
                let mut char_name = String::new();
                loop {
                    match read!() {
                        b'}' => break,
                        n => match from_u32(n as u32) {
                            None => return Err(Error::InvalidValue),
                            Some(c) => char_name.push(c),
                        }
                    }
                }
                match character(&char_name) {
                    None => return Err(Error::InvalidValue),
                    Some(character) => push_char!(character),
                }

            },
            _ => {
                buf.push(b'\\');
                buf.push(marker);
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use super::{unescape};

    #[test]
    fn test_unescape() {
        assert_eq!(unescape(b"foo", false).unwrap(), b"foo");
        assert_eq!(unescape(b"f\\noo", false).unwrap(), b"f\noo");
        assert_eq!(unescape(b"f\\x01oo", false).unwrap(), b"f\x01oo");
        assert_eq!(unescape(b"f\\375oo", false).unwrap(), b"f\xfdoo");
        assert_eq!(unescape(b"f\\75oo", false).unwrap(), b"f\x3doo");
        assert_eq!(unescape(b"f\\5oo", false).unwrap(), b"f\x05oo");
        assert_eq!(unescape(b"f\\oo", false).unwrap(), b"f\\oo");
        assert_eq!(unescape(b"f\\coo", false).unwrap(), b"f\\coo");
        assert_eq!(unescape(b"f\\U00002663oo", true).unwrap(), b"f\xe2\x99\xa3oo");
        assert_eq!(unescape(b"f\\u2663oo", true).unwrap(), b"f\xe2\x99\xa3oo");
        assert_eq!(unescape(b"f\\N{SNOWMAN}oo", true).unwrap(), b"f\xe2\x98\x83oo");
    }
}
