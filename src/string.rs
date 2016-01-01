use std::collections::{VecDeque};

quick_error! {
    #[derive(Debug)]
    pub enum Error {
        InvalidHexValue(c: u8)
        InvalidOctValue(c: u8)
        UnexpectedEnd
    }
}

fn hex_to_digit(c: u8) -> Result<u8, Error> {
    Ok(match c {
        b'0' ... b'9' => c - b'0',
        b'a' ... b'f' => c - b'a' + 10,
        b'A' ... b'F' => c - b'A' + 10,
        _ => return Err(Error::InvalidHexValue(c)),
    })
}

fn oct_to_digit(c: u8) -> Result<u8, Error> {
    Ok(match c {
        b'0' ... b'7' => c - b'0',
        _ => return Err(Error::InvalidOctValue(c)),
    })
}

fn unescape(s: &[u8]) -> Result<Vec<u8>, Error> {
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
                let first = try!(hex_to_digit(read!()));
                let second = try!(hex_to_digit(read!()));
                buf.push(first * 15 + second)
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
        assert_eq!(unescape(b"foo").unwrap(), b"foo");
        assert_eq!(unescape(b"f\\noo").unwrap(), b"f\noo");
        assert_eq!(unescape(b"f\\x01oo").unwrap(), b"f\x01oo");
        assert_eq!(unescape(b"f\\375oo").unwrap(), b"f\xfdoo");
        assert_eq!(unescape(b"f\\75oo").unwrap(), b"f\x3doo");
        assert_eq!(unescape(b"f\\5oo").unwrap(), b"f\x05oo");
        assert_eq!(unescape(b"f\\oo").unwrap(), b"f\\oo");
        assert_eq!(unescape(b"f\\coo").unwrap(), b"f\\coo");
    }
}