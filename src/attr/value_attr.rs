use std::str::FromStr;
use std::error::Error;

type DynoError = Box<dyn Error>;
const VALUE_ATTR_IDENTIFIER:[&str; 3] = ["ST/X", "ST/Y", "WERT"];

pub enum ValueAttr {
    WERT(Vec<f64>),
    STX(Vec<f64>),
    STY(Vec<f64>),
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
                Ok(Self::WERT(values))},
            Some("STX") => {
                let mut values = Vec::<f64>::new();
                for val in line.split_whitespace().skip(1) {
                    values.push(val.parse()?);
                }
                Ok(Self::STX(values))},
            Some("STY") => {
                let mut values = Vec::<f64>::new();
                for val in line.split_whitespace().skip(1) {
                    values.push(val.parse()?);
                }
                Ok(Self::STY(values))},
            _ => Err("Unknown line format".into())
        }
    }
}

impl From<ValueAttr> for Vec<f64> {
    fn from(value: ValueAttr) -> Self {
        match value {
            ValueAttr::WERT(v) => v,
            ValueAttr::STX(v) => v,
            ValueAttr::STY(v) => v,
        }
    }
}

pub fn concatenate(left: &ValueAttr, right: &ValueAttr) -> Result<Vec<f64>, DynoError> {
    match (left, right) {
        (ValueAttr::WERT(l), ValueAttr::WERT(r)) => Ok(l.iter().chain(r.iter()).cloned().collect()),
        (ValueAttr::STX(l), ValueAttr::STX(r)) => Ok(l.iter().chain(r.iter()).cloned().collect()),
        (ValueAttr::STY(l), ValueAttr::STY(r)) => Ok(l.iter().chain(r.iter()).cloned().collect()),
        _ => Err("Cannot concatenate different types of value attributes".into())
    }
}


fn get_line_first_word(s: &str) -> Option<&str> {
    s.split_once(" ")
            .and_then(|(first, _)| Some(first))
}

pub fn is_value_attr_line(s: &str) -> bool {
    s.trim().split_whitespace().nth(0)
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
        let attr:Vec<f64> = line.parse::<ValueAttr>()?.into();
        assert_eq!(attr, vec![1.0, 2.0, 3.0]);
        Ok(())
    }
}