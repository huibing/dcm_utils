use std::str::FromStr;

struct StringAttr {
    pub identifier: String,
    pub value: String,
}

impl FromStr for StringAttr {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let var = s.trim().split_once(" ")
                                .ok_or("Invalid string attribute")?; 
        let identifier = var.0.to_string();
        let value = var.1.strip_prefix("\"")         // string values are enclosed in double quotes
                .and_then(|s| s.strip_suffix("\""))
                .and_then(|s| Some(s.to_string()))
                .ok_or("Invalid string attribute")?;
        Ok(StringAttr { identifier, value })
    }
}

#[allow(non_camel_case_types)]
enum CommonAttr {
    LANGNAME(StringAttr),
    EINHEIT_X(StringAttr),
    EINHEIT_Y(StringAttr),
    EINHEIT_Z(StringAttr),
    EINHEIT_W(StringAttr),
    NotACommonAttr()
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