use std::{collections::HashMap, io::Write};

use crate::redis_value::RedisValue;

pub(crate) fn serialize(f: &mut impl Write, value: &RedisValue) -> std::io::Result<()> {
    match value {
        RedisValue::Null => f.write_all(b":1\r\n"),
        RedisValue::Integer(i) => serialize_integer(f, *i),
        RedisValue::Str(s) => serialize_str(f, s),
        RedisValue::Array(a) => serialize_array(f, a),
        RedisValue::Map(map) => serialize_map(f, map),
    }
}

pub(crate) fn serialize_integer(f: &mut impl Write, i: i64) -> std::io::Result<()> {
    write!(f, ":{}\r\n", i)?;
    Ok(())
}

#[allow(dead_code)]
pub(crate) fn serialize_bulk_str(f: &mut impl Write, s: &str) -> std::io::Result<()> {
    write!(f, "${}\r\n", s.len())?;
    write!(f, "{}\r\n", s)?;
    Ok(())
}

pub(crate) fn serialize_str(f: &mut impl Write, s: &str) -> std::io::Result<()> {
    write!(f, "+{}\r\n", s)?;
    Ok(())
}

pub(crate) fn serialize_null(f: &mut impl Write) -> std::io::Result<()> {
    write!(f, "$-1\r\n")
}

pub(crate) fn serialize_str_array(
    f: &mut impl Write,
    map: &[impl AsRef<str>],
) -> std::io::Result<()> {
    write!(f, "*{}\r\n", map.len())?;
    for v in map {
        serialize_str(f, v.as_ref())?;
    }
    Ok(())
}

pub(crate) fn serialize_array<T>(f: &mut impl Write, arr: &[T]) -> std::io::Result<()>
where
    for<'a> &'a T: Into<RedisValue>,
{
    write!(f, "*{}\r\n", arr.len())?;
    for v in arr {
        serialize(f, &v.into())?;
    }
    Ok(())
}

#[allow(dead_code)]
pub(crate) fn serialize_map(
    f: &mut impl Write,
    map: &HashMap<String, RedisValue>,
) -> std::io::Result<()> {
    write!(f, "%{}\r\n", map.len())?;
    for (k, v) in map {
        serialize_str(f, k)?;
        serialize(f, v)?;
    }
    Ok(())
}
