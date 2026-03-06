use std::str::FromStr;
use crate::attr::attr_arbitor::Attr;
use crate::attr::string_attr::StringAttr;
use crate::attr::value_attr::ValueAttr;
use crate::value::Value;
use crate::AxisType;
use log::warn;


#[derive(Clone)]
pub struct GRUPPENKENNFELD {
    pub name: String,
    pub attrs: Vec<StringAttr>,
    pub value: Vec<Value>,
    pub value_flat: Value,
    pub x_axis_name: String,
    pub y_axis_name: String,
    pub dim: (usize, usize),
    pub x_axis: Vec<f64>,
    pub y_axis: Vec<f64>,
}

impl FromStr for GRUPPENKENNFELD {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut lines = s.lines();
        let mut value_holder: Value = Value::new();
        let mut attrs: Vec<StringAttr> = Vec::new();
        let mut x_axis: Vec<f64> = Vec::new();
        let mut y_axis: Vec<f64> = Vec::new();
        let mut x_axis_name = String::new();
        let mut y_axis_name = String::new();
        let line = lines.next().ok_or("no first line found in GRUPPENKENNFELD")?;
        let name = line.split_whitespace().nth(1).ok_or("no name found in GRUPPENKENNFELD")?.to_string();
        let ncol = line.split_whitespace().nth(2)
                .ok_or("no ncol found in GRUPPENKENNFELD")?.parse::<usize>().map_err(|_| "ncol is not a number")?;
        let nrow = line.split_whitespace().nth(3)
                .ok_or("no nrow found in GRUPPENKENNFELD")?.parse::<usize>().map_err(|_| "nrow is not a number")?;
        let dim = (ncol, nrow);
        for line in lines {
            match line.parse::<Attr>() {
                Ok(Attr::StringAttr(s)) => attrs.push(s),
                Ok(Attr::ValueAttr(v)) => {
                    if let ValueAttr::WERT(w) = v {
                        value_holder.extend_f64(w);
                    } else if let ValueAttr::STX(x) = v {
                        x_axis.extend(x)
                    } else if let ValueAttr::STY(y) = v {
                        y_axis.extend(y)
                    } else if let ValueAttr::TEXT(t) = v {
                        value_holder.extend_string(t)
                    }
                },
                Ok(Attr::EmptyLine) => {},
                Ok(Attr::AxisVar(a)) => {
                    if  a.axistype == AxisType::X {
                        x_axis_name = a.identifier;
                    } else if a.axistype == AxisType::Y {
                        y_axis_name = a.identifier;
                    } else {
                        warn!("unknown axis attr: {}", a.identifier);
                    }
                },
                Err(e) => {
                    warn!("error parsing line: {}", e);
                    break;
                }
            }
        }
        let value_flat = value_holder.clone();
        let value = match value_holder {
            Value::TEXT(t) => {
                t.chunks(ncol).map(|v| Value::TEXT(v.to_vec())).collect()
            },
            Value::WERT(w) => {
                w.chunks(ncol).map(|v| Value::WERT(v.to_vec())).collect()
            }
        };
        Ok( Self {
            name,
            dim,
            value,
            x_axis,
            y_axis,
            x_axis_name,
            y_axis_name,
            attrs,
            value_flat
        })
    }
}

impl GRUPPENKENNFELD {
    pub fn from_f64(name: &str, value: Vec<Vec<f64>>, x_axis: Vec<f64>, 
        y_axis: Vec<f64>, x_axis_name: &str, y_axis_name: &str, 
        desc: &str, unit_w: &str, unit_x: &str, unit_y: &str) -> Self {
        let dim = (x_axis.len(), y_axis.len());
        let value_flat = Value::WERT(value.iter().flat_map(|v| v.clone()).collect());
        let attrs = vec![StringAttr::new("LANGNAME", desc),
                         StringAttr::new("EINHEIT_W", unit_w),
                         StringAttr::new("EINHEIT_X", unit_x),
                         StringAttr::new("EINHEIT_Y", unit_y)];
        let value = value.into_iter().map(|v| Value::WERT(v)).collect();
        Self {
            value,
            dim,
            name: name.to_string(),
            x_axis,
            y_axis,
            x_axis_name: x_axis_name.to_string(),
            y_axis_name: y_axis_name.to_string(),
            attrs,
            value_flat
        }
    }

    pub fn from_string(name: String, value: Vec<Vec<String>>, x_axis: Vec<f64>, 
        y_axis: Vec<f64>, x_axis_name: String, y_axis_name: String, 
        desc: String, unit_w: String, unit_x: String, unit_y: String) -> Self {
        let dim = (x_axis.len(), y_axis.len());
        let value_flat = Value::TEXT(value.iter().flat_map(|v| v.clone()).collect());
        let attrs = vec![StringAttr::new("LANGNAME", desc.as_str()),
                         StringAttr::new("EINHEIT_W", unit_w.as_str()),
                         StringAttr::new("EINHEIT_X", unit_x.as_str()),
                         StringAttr::new("EINHEIT_Y", unit_y.as_str())];
        let value = value.into_iter().map(|v| Value::TEXT(v)).collect();
        Self {
            value,
            dim,
            name,
            x_axis,
            y_axis,
            x_axis_name,
            y_axis_name,
            attrs,
            value_flat
        }
    }
}

impl PartialEq for GRUPPENKENNFELD {
    fn eq(&self, other: &Self) -> bool {
        self.value_flat == other.value_flat && 
        self.x_axis == other.x_axis && 
        self.y_axis == other.y_axis
    }
}
