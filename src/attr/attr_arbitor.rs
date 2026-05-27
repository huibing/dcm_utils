use super::string_attr::{is_axis_var, is_string_attr, AxisVar, StringAttr};
use super::value_attr::{is_value_attr_line, ValueAttr};
use std::str::FromStr;

pub enum Attr {
    StringAttr(StringAttr),
    AxisVar(AxisVar),
    ValueAttr(ValueAttr), // TODO: Add more types here
    EmptyLine,            // TODO: Add more types here
}

impl FromStr for Attr {
    type Err = Box<dyn std::error::Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if is_string_attr(s) {
            Ok(Attr::StringAttr(s.parse()?))
        } else if is_value_attr_line(s) {
            Ok(Attr::ValueAttr(s.parse()?))
        } else if is_axis_var(s) {
            Ok(Attr::AxisVar(s.parse()?))
        } else if s == "*SST" {
            Ok(Attr::EmptyLine)
        } else {
            Err("Invalid attribute".into())
        }
    }
}
