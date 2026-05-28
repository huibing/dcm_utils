use crate::attr::attr_arbitor::Attr;
use crate::attr::string_attr::StringAttr;
use crate::attr::value_attr::ValueAttr;
use crate::value::Value;
use log::{info, warn};
use std::str::FromStr;

#[derive(Clone, Debug)]
pub struct STUETZSTELLENVERTEILUNG {
    pub name: String,
    pub attrs: Vec<StringAttr>,
    pub value: Value,
    pub dim: usize,
    pub source_path: Option<String>,
    pub source_line: usize,
}

impl STUETZSTELLENVERTEILUNG {
    pub fn parse_with_context(
        s: &str,
        source_path: Option<&str>,
        source_line: usize,
    ) -> Result<Self, &'static str> {
        let mut lines = s.lines();
        let mut attrs = Vec::new();
        let mut value: Value = Value::new();
        let first_line = lines
            .next()
            .ok_or("no first line found in STUETZSTELLENVERTEILUNG")?;
        let name = first_line
            .split_whitespace()
            .nth(1)
            .ok_or("no name found in STUETZSTELLENVERTEILUNG")?
            .to_string();
        let dim = first_line
            .split_whitespace()
            .nth(2)
            .ok_or("no dim found in STUETZSTELLENVERTEILUNG")?
            .parse::<usize>()
            .unwrap();
        for (i, line) in lines.enumerate() {
            let abs_line = source_line + i + 1;
            match line.parse::<Attr>() {
                Ok(Attr::StringAttr(sa)) => attrs.push(sa),
                Ok(Attr::ValueAttr(va)) => {
                    if let ValueAttr::STX(w) = va {
                        value.extend_f64(w);
                    } else {
                        return Err("unknown value type");
                    }
                }
                Ok(Attr::EmptyLine) => {}
                Ok(Attr::AxisVar(_)) => {
                    if let Some(path) = source_path {
                        warn!("[{}:{}] STUETZSTELLENVERTEILUNG shall not have axis var line: {}", path, abs_line, line);
                    } else {
                        warn!(
                            "STUETZSTELLENVERTEILUNG shall not have axis var line: {}",
                            line
                        );
                    }
                }
                Err(error_msg) => {
                    if let Some(path) = source_path {
                        warn!("[{}:{}] error parsing line: {}, error: {}", path, abs_line, line, error_msg);
                    } else {
                        info!("error parsing line: {}, error: {}", line, error_msg);
                    }
                }
            }
        }
        Ok(Self {
            name,
            attrs,
            value,
            dim,
            source_path: source_path.map(String::from),
            source_line,
        })
    }
}

impl FromStr for STUETZSTELLENVERTEILUNG {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse_with_context(s, None, 0)
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
            attrs: vec![
                StringAttr::new("LANGNAME", desc),
                StringAttr::new("EINHEIT_X", unit),
            ],
            value,
            dim,
            source_path: None,
            source_line: 0,
        }
    }
}
