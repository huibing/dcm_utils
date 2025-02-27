use std::str::FromStr;
use std::error::Error;
use crate::attr::attr_arbitor::Attr;
use crate::attr::string_attr::StringAttr;
use crate::value::Value;
use log::{warn, info};


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
        let mut first_line_words = lines.nth(0).unwrap().trim().split_whitespace();
        let name = first_line_words.nth(1)
                        .ok_or::<&str>("no name found".into())?.to_string();
        let dim = first_line_words.next()
                        .ok_or::<&str>("no dim found".into())?.parse::<usize>()?;
        for line in lines {
            match line.parse::<Attr>() {
                Ok(Attr::StringAttr(sa)) => attrs.push(sa),
                Ok(Attr::ValueAttr(v)) => value = v.into(),
                Ok(Attr::AxisVar(_)) => {
                    info!("ignoring axis var in FESTWERT");
                },
                Ok(Attr::EmptyLine) => {},
                Err(err_msg) => {
                    warn!("error {} parsing line: {}", err_msg, line);
                }
            }
        }
        if value.len() == 0 {
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