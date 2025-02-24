use std::str::FromStr;
use crate::AxisType;

const STRING_ATTR_IDENTIFIER: [&'static str; 4] = ["LANGNAME", "EINHEIT_X", "EINHEIT_Y", "EINHEIT_W"];

#[allow(dead_code)]
pub struct StringAttr {
    pub identifier: String,
    pub value: String,
}

impl FromStr for StringAttr {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let words = s.trim().split_once(" ").ok_or("Invalid string attribute")?;
        if !STRING_ATTR_IDENTIFIER.contains(&words.0) {
            return Err("Invalid identifier");
        }
        let identifier = words.0.to_string();
        let value = words.1.strip_prefix("\"")         // string values are enclosed in double quotes
                .and_then(|s| s.strip_suffix("\""))
                .and_then(|s| Some(s.to_string()))
                .ok_or("Invalid string attribute")?;
        Ok(StringAttr { identifier, value })
    }
}

pub fn is_string_attr(s: &str) -> bool {
    s.trim().split_whitespace().nth(0).map(|s| STRING_ATTR_IDENTIFIER.contains(&s)).unwrap_or(false)
}

pub fn is_axis_var(s: &str) -> bool {
    s.trim().split_whitespace().next().map(|s| s == "*SSTY" || s == "*SSTY").unwrap_or(false)
}

pub struct AxisVar {
    pub axistype: AxisType,
    pub identifier: String,
}

impl FromStr for AxisVar {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let words = s.trim().split_whitespace().collect::<Vec<&str>>();
        if words.len() != 2 {
            return Err("Invalid axis variable");
        }
        let axistype = if words[0] == "*SSTX" {
            AxisType::X
        } else if words[0] == "*SSTY" {
            AxisType::Y
        } else {
            return Err("Invalid axis type identifier");
        };
        Ok( Self{axistype, identifier: words[1].to_string() })
    }
}

#[cfg(test)]
pub mod string_attr_tests {
    use super::*;
    use rstest::*;

    #[rstest]
    fn test_string_attr() {
        let s = "EINHEIT_W \"unitless\"";
        let a = s.parse::<StringAttr>().unwrap();
        assert_eq!(a.identifier, "EINHEIT_W");
        assert_eq!(a.value, "unitless");
    }

    #[rstest]
    #[case("EINHEIT_W \"unitless\"", "EINHEIT_W", "unitless")]
    #[case("LANGNAME \"\"", "LANGNAME", "")]
    #[case("LANGNAME \"[Cmft mode speed leveling threshold when exceeds, height level lows from LL1 to LL2\"", "LANGNAME", "[Cmft mode speed leveling threshold when exceeds, height level lows from LL1 to LL2")]
    fn test_string_attr_cases(#[case] s: &str, #[case] identifier: &str, #[case] value: &str) {
        let a = s.parse::<StringAttr>().unwrap();
        assert_eq!(a.identifier, identifier);
        assert_eq!(a.value, value);
    }
}