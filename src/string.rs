use std::collections::{VecDeque};
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

fn oct_to_digit(c: u8) -> Result<u8, Error> {
    Ok(match c {
        b'0' ... b'7' => c - b'0',
        _ => return Err(Error::InvalidValue),
    })
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
            s.get(i).map(|c| *c)
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
                oct_buf.push_front(try!(oct_to_digit(marker)));
                peek!().and_then(|c| oct_to_digit(c).ok()).map(|s| {
                    oct_buf.push_front(s);
                    i += 1;

                    peek!().and_then(|c| oct_to_digit(c).ok()).map(|s| {
                        oct_buf.push_front(s);
                        i += 1;
                    });
                });
                let value = oct_buf.iter().enumerate().fold(0u16, |acc, (i, &v)| acc + v as u16 * (8u16.pow(i as u32)));
                oct_buf.clear();
                buf.push(if value > 255 {
                    255
                } else {
                    value as u8
                });
                continue
            },
            b'u' if unicode => unimplemented!(),
            b'U' if unicode => unimplemented!(),
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
    }
}