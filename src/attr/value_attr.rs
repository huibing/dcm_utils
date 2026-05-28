use std::error::Error;
use std::str::FromStr;

type DynoError = Box<dyn Error>;
const VALUE_ATTR_IDENTIFIER: [&str; 4] = ["ST/X", "ST/Y", "WERT", "TEXT"];

pub enum ValueAttr {
    WERT(Vec<f64>),
    STX(Vec<f64>),
    STY(Vec<f64>),
    TEXT(Vec<String>),
}

impl FromStr for ValueAttr {
    type Err = DynoError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let line = s.trim();
        match get_line_first_word(line) {
            Some("WERT") => {
                let mut values = Vec::<f64>::new();
                for val in line.split_whitespace().skip(1) {
                    values.push(val.parse()?);
                }
                Ok(Self::WERT(values))
            }
            Some("ST/X") => {
                let mut values = Vec::<f64>::new();
                for val in line.split_whitespace().skip(1) {
                    values.push(val.parse()?);
                }
                Ok(Self::STX(values))
            }
            Some("ST/Y") => {
                let mut values = Vec::<f64>::new();
                for val in line.split_whitespace().skip(1) {
                    values.push(val.parse()?);
                }
                Ok(Self::STY(values))
            }
            Some("TEXT") => {
                let mut values = Vec::<String>::new();
                for token in split_whitespace_quoted(line).into_iter().skip(1) {
                    let text = token
                        .strip_prefix("\"")
                        .and_then(|s| s.strip_suffix("\""))
                        .unwrap_or(token);
                    values.push(text.to_string());
                }
                Ok(Self::TEXT(values))
            }
            _ => Err("Unknown line format".into()),
        }
    }
}

impl From<ValueAttr> for Vec<f64> {
    fn from(value: ValueAttr) -> Self {
        match value {
            ValueAttr::WERT(v) => v,
            ValueAttr::STX(v) => v,
            ValueAttr::STY(v) => v,
            ValueAttr::TEXT(_) => vec![f64::NAN], // not a number
        }
    }
}

pub fn concatenate(left: &ValueAttr, right: &ValueAttr) -> Result<Vec<f64>, DynoError> {
    match (left, right) {
        (ValueAttr::WERT(l), ValueAttr::WERT(r)) => Ok(l.iter().chain(r.iter()).cloned().collect()),
        (ValueAttr::STX(l), ValueAttr::STX(r)) => Ok(l.iter().chain(r.iter()).cloned().collect()),
        (ValueAttr::STY(l), ValueAttr::STY(r)) => Ok(l.iter().chain(r.iter()).cloned().collect()),
        _ => Err("Cannot concatenate different types of value attributes".into()),
    }
}

/// Split a string on whitespace, treating double-quoted substrings as single tokens.
/// e.g. `TEXT   "foo bar"   baz` → `["TEXT", "\"foo bar\"", "baz"]`
fn split_whitespace_quoted(input: &str) -> Vec<&str> {
    let mut tokens: Vec<&str> = Vec::new();
    let mut start = 0;
    let mut in_token = false;
    let mut in_quotes = false;
    let bytes = input.as_bytes();
    for i in 0..bytes.len() {
        let c = bytes[i] as char;
        if c == '"' {
            if !in_token {
                start = i;
                in_token = true;
            }
            in_quotes = !in_quotes;
            continue;
        }
        if in_quotes {
            continue;
        }
        if c.is_whitespace() {
            if in_token {
                tokens.push(&input[start..i]);
                in_token = false;
            }
        } else if !in_token {
            start = i;
            in_token = true;
        }
    }
    if in_token {
        tokens.push(&input[start..]);
    }
    tokens
}

fn get_line_first_word(s: &str) -> Option<&str> {
    s.split_once(" ").map(|(first, _)| first)
}

pub fn is_value_attr_line(s: &str) -> bool {
    s.split_whitespace()
        .next()
        .map(|word| VALUE_ATTR_IDENTIFIER.contains(&word))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    #[rstest]
    fn test_parse_value_attr() -> Result<(), DynoError> {
        let line = "WERT 1.0 2.0 3.0";
        let attr: Vec<f64> = line.parse::<ValueAttr>()?.into();
        assert_eq!(attr, vec![1.0, 2.0, 3.0]);
        Ok(())
    }

    #[rstest]
    fn test_parse_text_quoted_spaces() -> Result<(), DynoError> {
        // Quoted string with spaces should be a single token
        let line = "TEXT   \"P0425519 OF\"";
        let attr = line.parse::<ValueAttr>()?;
        match attr {
            ValueAttr::TEXT(v) => {
                assert_eq!(v.len(), 1, "quoted string with spaces should be one token");
                assert_eq!(v[0], "P0425519 OF");
            }
            _ => panic!("expected TEXT"),
        }
        Ok(())
    }

    #[rstest]
    fn test_parse_text_multiple_quoted() -> Result<(), DynoError> {
        // Multiple quoted strings
        let line = "TEXT   \"value one\"   \"value two\"";
        let attr = line.parse::<ValueAttr>()?;
        match attr {
            ValueAttr::TEXT(v) => {
                assert_eq!(v.len(), 2);
                assert_eq!(v[0], "value one");
                assert_eq!(v[1], "value two");
            }
            _ => panic!("expected TEXT"),
        }
        Ok(())
    }

    #[rstest]
    fn test_parse_text_mixed_quoted_and_unquoted() -> Result<(), DynoError> {
        // Mixed quoted and unquoted tokens
        let line = "TEXT   simple   \"quoted value\"   plain";
        let attr = line.parse::<ValueAttr>()?;
        match attr {
            ValueAttr::TEXT(v) => {
                assert_eq!(v.len(), 3);
                assert_eq!(v[0], "simple");
                assert_eq!(v[1], "quoted value");
                assert_eq!(v[2], "plain");
            }
            _ => panic!("expected TEXT"),
        }
        Ok(())
    }
}
