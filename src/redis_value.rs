use std::collections::HashMap;

#[derive(Debug)]
pub(crate) enum RedisValue {
    Null,
    Integer(i64),
    Str(String),
    Array(Vec<RedisValue>),
    Map(HashMap<String, RedisValue>),
}

impl RedisValue {
    pub(crate) fn as_str(&self) -> Option<&str> {
        match self {
            Self::Str(s) => Some(s),
            _ => None,
        }
    }
}
