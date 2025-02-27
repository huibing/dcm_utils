use log::warn;
use crate::attr::value_attr::ValueAttr;
use std::error::Error;

#[derive(Debug, Clone)]
pub enum Value {
    WERT(Vec<f64>),
    TEXT(Vec<String>),
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
            (Value::WERT(v1), Value::WERT(v2)) => v1 == v2,
            (Value::TEXT(v1), Value::TEXT(v2)) => v1 == v2,
            _ => false,
        }
    }
}