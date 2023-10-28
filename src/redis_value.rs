use std::collections::HashMap;

#[derive(Debug, Clone)]
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

impl From<&RedisValue> for RedisValue {
    fn from(value: &RedisValue) -> Self {
        value.clone()
    }
}

impl From<&str> for RedisValue {
    fn from(value: &str) -> Self {
        RedisValue::Str(value.to_string())
    }
}

impl From<&&str> for RedisValue {
    fn from(value: &&str) -> Self {
        RedisValue::Str(value.to_string())
    }
}
