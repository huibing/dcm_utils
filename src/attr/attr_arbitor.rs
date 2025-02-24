use super::string_attr::{StringAttr, AxisVar, is_string_attr, is_axis_var};
use super::value_attr::{ValueAttr, is_value_attr_line};
use std::str::FromStr;


pub enum Attr {
    StringAttr(StringAttr),
    AxisVar(AxisVar),
    ValueAttr(ValueAttr), // TODO: Add more types here
}

impl FromStr for Attr {
    type Err = Box<dyn std::error::Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if is_string_attr(s) {
            Ok(Attr::StringAttr(s.parse()?))
        } else if is_value_attr_line(s) {
            Ok(Attr::ValueAttr(s.parse()?))
        } else if is_axis_var(s){
            Ok(Attr::AxisVar(s.parse()?))
        } else {
            Err("Invalid attribute".into())
        }
    }
}

