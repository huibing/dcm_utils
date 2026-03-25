use std::str::FromStr;
use std::error::Error;
use crate::attr::attr_arbitor::Attr;
use crate::attr::string_attr::StringAttr;
use crate::value::Value;
use log::{warn, info};
use crate::attr::value_attr::ValueAttr;


#[derive(Clone)]
pub struct FESTWERTEBLOCK {
    pub attrs: Vec<StringAttr>,
    pub value: Value,       // for FESTWERT, only one value in the vector
    pub name: String,
    pub dim: usize,
}

impl FromStr for FESTWERTEBLOCK {
    type Err=Box<dyn Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut attrs: Vec<StringAttr> = Vec::new();
        let mut lines = s.lines();
        let mut value: Value = Value::new();
        let mut first_line_words = lines.nth(0).unwrap().split_whitespace();
        let name = first_line_words.nth(1)
                        .ok_or::<&str>("no name found")?.to_string();
        let dim = first_line_words.next()
                        .ok_or::<&str>("no dim found")?.parse::<usize>()?;
        for line in lines {
            match line.parse::<Attr>() {
                Ok(Attr::StringAttr(sa)) => attrs.push(sa),
                Ok(Attr::ValueAttr(v)) => {
                    if let ValueAttr::WERT(w) = v {
                        value.extend_f64(w);
                    } else if let ValueAttr::TEXT(t) = v {
                        value.extend_string(t);
                    }
                },
                Ok(Attr::AxisVar(_)) => {
                    info!("ignoring axis var in FESTWERT");
                },
                Ok(Attr::EmptyLine) => {},
                Err(err_msg) => {
                    warn!("error {} parsing line: {}", err_msg, line);
                }
            }
        }
        if value.is_empty() {
            return Err(format!("no value found in FESTWERT {name}").into());
        }
        Ok(FESTWERTEBLOCK {
            name,
            attrs,
            value,
            dim,
        })
    }
}

impl PartialEq for FESTWERTEBLOCK {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl FESTWERTEBLOCK {
    pub fn from_f64(name: String, value: Vec<f64>, desc: String, unit: String) -> Self {
        let dim = value.len();
        let value = Value::WERT(value);
        Self {
            name,
            value,
            attrs: vec![StringAttr::new("LANGNAME", desc.as_str()),
                        StringAttr::new("EINHEIT_W", unit.as_str())],
            dim,
        }
    }

    pub fn from_string(name: String, value: Vec<String>, desc: String, unit: String) -> Self {
        let dim = value.len();
        let value = Value::TEXT(value);
        Self {
            name,
            value,
            attrs: vec![StringAttr::new("LANGNAME", desc.as_str()),
                        StringAttr::new("EINHEIT_W", unit.as_str())],
            dim,
        }
    }
}