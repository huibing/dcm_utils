use log::warn;
use crate::attr::value_attr::ValueAttr;
use std::error::Error;
use serde::{Serialize, Serializer, Deserialize, Deserializer};
use serde::de::{self, Visitor, SeqAccess};
use std::fmt;

#[derive(Debug, Clone)]
pub enum Value {
    WERT(Vec<f64>),
    TEXT(Vec<String>),
}

impl Default for Value {
    fn default() -> Self {
        Self::new()
    }
}

impl Value {

    pub fn new() -> Self {
        Value::WERT(Vec::new())
    }

    pub fn len(&self) -> usize {
        match self {
            Value::WERT(v) => v.len(),
            Value::TEXT(v) => v.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Value::WERT(v) => v.is_empty(),
            Value::TEXT(v) => v.is_empty(),
        }
    }

    pub fn extend_f64(&mut self, value: Vec<f64>) {
        match self {
            Value::WERT(v) => v.extend(value),
            Value::TEXT(_) => {
                warn!("cannot extend f64 to TEXT");
                *self = Self::WERT(value);
            }
        }
    }

    pub fn extend_string(&mut self, value: Vec<String>) {
        match self {
            Value::WERT(_) => {
                warn!("cannot extend string to f64, will renew the value");
                *self = Self::TEXT(value);
            }
            Value::TEXT(v) => v.extend(value),
        }
    }

    pub fn try_into_f64(&self) -> Result<&Vec<f64>, Box<dyn Error>> {
        match self {
            Self::WERT(v) => Ok(v),
            Self::TEXT(_) => Err("cannot convert TEXT to f64".into()),
        }
    }
}

impl From<ValueAttr> for Value {

    fn from(value: ValueAttr) -> Self {
        match value {
            ValueAttr::WERT(v) => Value::WERT(v),
            ValueAttr::TEXT(v) => Value::TEXT(v),
            _ => Value::WERT(vec![]),  // will not come here
        }
    }
}

impl PartialEq for Value {
    
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::WERT(v1), Value::WERT(v2)) => {
                if v1.len() != v2.len() {
                    return false;
                }
                v1.iter().zip(v2.iter()).all(|(a, b)| a == b)
            },
            (Value::TEXT(v1), Value::TEXT(v2)) => v1 == v2,
            _ => false,
        }
    }
}

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Value::WERT(values) => {
                // 序列化 Vec<f64> 为数字数组
                values.serialize(serializer)
            }
            Value::TEXT(values) => {
                // 序列化 Vec<String> 为字符串数组
                values.serialize(serializer)
            }
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::WERT(values) => {
                write!(f, "WERT[{}]", values.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(", "))
            }
            Value::TEXT(values) => {
                write!(f, "TEXT[{}]", values.join(", "))
            }
        }
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ValueVisitor;

        impl<'de> Visitor<'de> for ValueVisitor {
            type Value = Value;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a sequence of numbers or strings")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                // Try to deserialize as Vec<f64> first
                let mut numbers: Vec<f64> = Vec::new();
                let mut strings: Vec<String> = Vec::new();
                let mut is_text = false;

                while let Some(value) = seq.next_element::<serde_json::Value>()? {
                    if let Some(num) = value.as_f64() {
                        if !is_text {
                            numbers.push(num);
                        } else {
                            strings.push(value.to_string());
                        }
                    } else if let Some(s) = value.as_str() {
                        is_text = true;
                        strings.push(s.to_string());
                    } else {
                        return Err(de::Error::custom("expected number or string"));
                    }
                }

                if is_text {
                    Ok(Value::TEXT(strings))
                } else {
                    Ok(Value::WERT(numbers))
                }
            }
        }

        deserializer.deserialize_seq(ValueVisitor)
    }
}

#[cfg(test)]
mod tests {
    use rstest::*;
    use super::*;
    use serde_json;

    #[rstest]
    fn test_value_serialize() {
        let value = Value::WERT(vec![1.0, 2.0, 3.0]);
        let expected_json = r#"[1.0,2.0,3.0]"#;
        let json = serde_json::to_string(&value).unwrap();
        assert_eq!(json, expected_json);
    }

    #[rstest]
    fn test_value_serialize_string() {
        let value = Value::TEXT(vec!["hello".to_string(), "not a number".to_string()]);
        let expected_json = r#"["hello","not a number"]"#;
        let json = serde_json::to_string(&value).unwrap();
        assert_eq!(json, expected_json);
    }
}