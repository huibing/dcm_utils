use std::str::FromStr;
use std::error::Error;
use crate::attr::attr_arbitor::Attr;
use crate::attr::string_attr::StringAttr;
use crate::value::Value;
use log::{warn, info};

pub struct FESTWERT {
    pub attrs: Vec<StringAttr>,
    pub value: Value,       // for FESTWERT, only one value in the vector
    pub name: String,
}

impl FromStr for FESTWERT {
    type Err=Box<dyn Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut attrs: Vec<StringAttr> = Vec::new();
        let mut lines = s.lines();
        let mut value: Value = Value::new();
        let name = lines.nth(0).unwrap().trim().split_whitespace().last()
                        .ok_or::<&str>("no name found".into())?.to_string();
        for line in lines {
            match line.parse::<Attr>() {
                Ok(Attr::StringAttr(sa)) => attrs.push(sa),
                Ok(Attr::ValueAttr(v)) => value = v.into(),
                Ok(Attr::AxisVar(_)) => {
                    info!("ignoring axis var in FESTWERT");
                }
                Ok(Attr::EmptyLine) => {}
                Err(err_msg) => {
                    warn!("error {} parsing line: {}", err_msg, line);
                }
            }
        }
        if value.len() == 0 {
            return Err(format!("no value found in FESTWERT {name}").into());
        }
        Ok(FESTWERT {
            name,
            attrs,
            value
        })
    }
}


impl FESTWERT {
    pub fn from_f64(name: String, value: f64, desc: String, unit: String) -> Self {
        let value = Value::WERT(vec![value]);
        Self {
            name,
            value,
            attrs: vec![StringAttr::new("LANGNAME", desc.as_str()),
                        StringAttr::new("EINHEIT_W", unit.as_str())],
        }
    }

    pub fn from_string(name: String, value: String, desc: String, unit: String) -> Self {
        let value = Value::TEXT(vec![value]);
        Self {
            name,
            value,
            attrs: vec![StringAttr::new("LANGNAME", desc.as_str()),
                        StringAttr::new("EINHEIT_W", unit.as_str())],
        }
    }
}