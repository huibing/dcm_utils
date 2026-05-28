use crate::attr::attr_arbitor::Attr;
use crate::attr::string_attr::StringAttr;
use crate::attr::value_attr::ValueAttr;
use crate::value::Value;
use crate::AxisType;
use log::{info, warn};
use std::str::FromStr;

#[derive(Clone, Debug)]
pub struct GRUPPENKENNLINIE {
    pub name: String,
    pub attrs: Vec<StringAttr>,
    pub value: Value,
    pub axis: Vec<f64>,
    pub axis_var_name: String,
    pub dim: usize,
    pub source_path: Option<String>,
    pub source_line: usize,
}

impl GRUPPENKENNLINIE {
    pub fn parse_with_context(
        s: &str,
        source_path: Option<&str>,
        source_line: usize,
    ) -> Result<Self, &'static str> {
        let mut lines = s.lines();
        let mut attrs = Vec::new();
        let mut value = Value::new();
        let mut axis = Vec::new();
        let mut axis_var_name = String::from("no_axis_var_name_found");
        let first_line = lines
            .next()
            .ok_or("no first line found in GRUPPENKENNLINIE")?;
        let name = first_line
            .split_whitespace()
            .nth(1)
            .ok_or("no name found")?
            .to_string();
        let dim = first_line
            .split_whitespace()
            .nth(2)
            .ok_or("no dim found")?
            .parse::<usize>()
            .unwrap();
        for (i, line) in lines.enumerate() {
            let abs_line = source_line + i + 1;
            match line.parse::<Attr>() {
                Ok(Attr::StringAttr(sa)) => attrs.push(sa),
                Ok(Attr::ValueAttr(va)) => {
                    if let ValueAttr::WERT(w) = va {
                        value.extend_f64(w);
                    } else if let ValueAttr::STX(sx) = va {
                        axis.extend(sx.into_iter());
                    } else if let ValueAttr::TEXT(t) = va {
                        value = Value::TEXT(t);
                        info!("found text value: {:?}", value);
                    } else {
                        return Err("unknown value type");
                    }
                }
                Ok(Attr::AxisVar(av)) => {
                    if av.axistype == AxisType::X {
                        axis_var_name = av.identifier;
                    }
                }
                Ok(Attr::EmptyLine) => {}
                Err(error_msg) => {
                    if let Some(path) = source_path {
                        warn!("[{}:{}] error parsing line: {}, error: {}", path, abs_line, line, error_msg);
                    } else {
                        warn!("error parsing line: {}, error: {}", line, error_msg);
                    }
                }
            }
        }
        Ok(GRUPPENKENNLINIE {
            name,
            attrs,
            value,
            axis,
            axis_var_name,
            dim,
            source_path: source_path.map(String::from),
            source_line,
        })
    }
}

impl FromStr for GRUPPENKENNLINIE {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse_with_context(s, None, 0)
    }
}

impl PartialEq for GRUPPENKENNLINIE {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
            && self.axis == other.axis
            && self.axis_var_name == other.axis_var_name
    }
}

impl GRUPPENKENNLINIE {
    pub fn from_f64(
        name: &str,
        value: &[f64],
        desc: &str,
        unit: &str,
        unit_x: &str,
        axis_var_name: &str,
        axis: &[f64],
    ) -> Self {
        let dim = value.len();
        let value = Value::WERT(value.to_owned());
        Self {
            name: name.to_string(),
            attrs: vec![
                StringAttr::new("LANGNAME", desc),
                StringAttr::new("EINHEIT_W", unit),
                StringAttr::new("EINHEIT_X", unit_x),
            ],
            value,
            axis: axis.to_owned(),
            axis_var_name: axis_var_name.to_string(),
            dim,
            source_path: None,
            source_line: 0,
        }
    }

    pub fn from_string(
        name: &str,
        value: Vec<String>,
        desc: String,
        unit: &str,
        unit_x: &str,
        axis_var_name: String,
        axis: Vec<f64>,
    ) -> Self {
        let dim = value.len();
        let value = Value::TEXT(value);
        Self {
            name: name.to_string(),
            attrs: vec![
                StringAttr::new("LANGNAME", desc.as_str()),
                StringAttr::new("EINHEIT_W", unit),
                StringAttr::new("EINHEIT_X", unit_x),
            ],
            value,
            axis,
            axis_var_name,
            dim,
            source_path: None,
            source_line: 0,
        }
    }
}
