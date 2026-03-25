use std::str::FromStr;
use crate::attr::attr_arbitor::Attr;
use crate::attr::string_attr::StringAttr;
use crate::attr::value_attr::ValueAttr;
use crate::value::Value;
use log::{warn, info};


#[derive(Clone)]
pub struct STUETZSTELLENVERTEILUNG {
    pub name: String,
    pub attrs: Vec<StringAttr>,
    pub value: Value,
    pub dim: usize,
}

impl FromStr for STUETZSTELLENVERTEILUNG {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut lines = s.lines();
        let mut attrs = Vec::new();
        let mut value: Value = Value::new();
        let first_line = lines.next().ok_or("no first line found in STUETZSTELLENVERTEILUNG")?;
        let name = first_line.split_whitespace().nth(1)
            .ok_or("no name found in STUETZSTELLENVERTEILUNG")?.to_string();
        let dim = first_line.split_whitespace().nth(2)
            .ok_or("no dim found in STUETZSTELLENVERTEILUNG")?.parse::<usize>().unwrap();
        for line in lines {
            match line.parse::<Attr>() {
                Ok(Attr::StringAttr(sa)) => attrs.push(sa),
                Ok(Attr::ValueAttr(va)) => {
                    if let ValueAttr::STX(w) = va{
                        value.extend_f64(w);
                    } else {
                        return Err("unknown value type");
                    }
                },
                Ok(Attr::EmptyLine) => {},
                Ok(Attr::AxisVar(_)) => {
                    warn!("STUETZSTELLENVERTEILUNG shall not have axis var line: {}", line);
                }
                Err(error_msg) => {
                    info!("error parsing line: {}, error: {}", line, error_msg);  //shall not stop the parser
                }
            }
        }
        Ok( Self {
            name,
            attrs,
            value,
            dim
        })
    }
}

impl PartialEq for STUETZSTELLENVERTEILUNG {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl STUETZSTELLENVERTEILUNG {
    pub fn from_f64(name: &str, desc: &str, value: &[f64], unit: &str) -> Self {
        let dim = value.len();
        let value = Value::WERT(value.to_owned());
        Self {
            name: String::from(name),
            attrs: vec![StringAttr::new("LANGNAME", desc),
                        StringAttr::new("EINHEIT_X", unit)],
            value,
            dim
        }
    }

}