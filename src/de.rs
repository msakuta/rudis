use std::{collections::HashMap, io::Read};

use crate::redis_value::RedisValue;

pub(crate) fn deserialize(f: &mut impl Read) -> Result<RedisValue, Box<dyn std::error::Error>> {
    let tag = read_byte(f)?;
    match tag {
        b':' => Ok(RedisValue::Integer(parse_len(f)? as i64)),
        b'$' => {
            // string
            let len = parse_len(f)?;
            // println!("str len: {len}");
            if len < 0 {
                return Ok(RedisValue::Null);
            }
            let mut str_buf = vec![0u8; len as usize];
            f.read_exact(&mut str_buf)?;
            if !matches!(read_byte(f), Ok(b'\r')) || !matches!(read_byte(f), Ok(b'\n')) {
                return Err("String not followed by a CRLF".into());
            }
            let str = String::from_utf8(str_buf)?;
            // println!("Str(utf-8): {str}");
            Ok(RedisValue::Str(str))
        }
        b'+' => {
            // simple string
            let mut str_buf = vec![0u8; 2];
            f.read_exact(&mut str_buf)?;
            loop {
                if str_buf[str_buf.len() - 2..] == b"\r\n"[..] {
                    return Ok(RedisValue::Str(
                        std::str::from_utf8(&str_buf[..str_buf.len() - 2])?.to_string(),
                    ));
                }
                let mut next_buf = [0u8; 1];
                let len = f.read(&mut next_buf)?;
                str_buf.extend_from_slice(&next_buf[..len]);
            }
        }
        b'*' => {
            // array
            let len = parse_len(f)?;
            // println!("array len: {len}");
            let v = (0..len).map(|_| deserialize(f)).collect::<Result<_, _>>()?;
            Ok(RedisValue::Array(v))
        }
        b'%' => {
            // map
            let len = parse_len(f)?;
            // println!("map len: {len}");
            let mut hash = HashMap::new();
            for _ in 0..len / 2 {
                let RedisValue::Str(k) = deserialize(f)? else {
                    return Err("Key is not a string".into());
                };
                // println!("key: {k:?}");
                let v = deserialize(f)?;
                hash.insert(k, v);
            }
            Ok(RedisValue::Map(hash))
        }
        _ => Err(format!("Not formatted as redis value: tag was {}", tag as char).into()),
    }
}

fn parse_len(f: &mut impl Read) -> Result<isize, Box<dyn std::error::Error>> {
    let mut len_buf = [b'0'; 16];
    let mut i = 0;
    let mut first_byte = None;
    let mut negative = false;
    while let Ok(b) = read_byte(f) {
        if b == b'-' {
            negative = true;
            continue;
        }
        if matches!(b, b'0'..=b'9') {
            len_buf[i] = b;
            i += 1;
        } else {
            first_byte = Some(b);
            break;
        }
    }
    if !matches!(first_byte, Some(b'\r')) || !matches!(read_byte(f), Ok(b'\n')) {
        return Err("Length not followed by a CRLF".into());
    }
    let len = std::str::from_utf8(&len_buf[..i])?.parse::<isize>()?;
    Ok(if negative { -len } else { len })
}

fn read_byte(f: &mut impl Read) -> std::io::Result<u8> {
    let mut buf = [0u8];
    f.read_exact(&mut buf)?;
    Ok(buf[0])
}
