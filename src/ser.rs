use std::collections::HashMap;

#[allow(dead_code)]
pub(crate) fn serialize_bulk_str(f: &mut impl std::io::Write, s: &str) -> std::io::Result<()> {
    write!(f, "${}\r\n", s.len())?;
    write!(f, "{}\r\n", s)?;
    Ok(())
}

pub(crate) fn serialize_str(f: &mut impl std::io::Write, s: &str) -> std::io::Result<()> {
    write!(f, "+{}\r\n", s)?;
    Ok(())
}

pub(crate) fn serialize_null(f: &mut impl std::io::Write) -> std::io::Result<()> {
    write!(f, "$-1\r\n")
}

pub(crate) fn serialize_array(
    f: &mut impl std::io::Write,
    map: &[impl AsRef<str>],
) -> std::io::Result<()> {
    write!(f, "*{}\r\n", map.len())?;
    for v in map {
        serialize_str(f, v.as_ref())?;
    }
    Ok(())
}

#[allow(dead_code)]
pub(crate) fn serialize_map(
    f: &mut impl std::io::Write,
    map: &HashMap<String, String>,
) -> std::io::Result<()> {
    write!(f, "%{}\r\n", map.len())?;
    for (k, v) in map {
        serialize_str(f, k)?;
        serialize_str(f, v)?;
    }
    Ok(())
}
