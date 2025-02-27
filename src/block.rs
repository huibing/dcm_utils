use crate::attr::string_attr::{StringAttr, eval_string_attr};
use crate::attr::value_attr::ValueAttr;
use crate::attr::attr_arbitor::Attr;
use crate::AxisType;
use std::str::FromStr;
use std::error::Error;
use log::{info, warn};


#[derive(Debug)]
pub enum Value {
    WERT(Vec<f64>),
    TEXT(Vec<String>),
}

impl Value {

    pub fn new() -> Self {
        Value::WERT(Vec::new())
    }

    pub fn len(&self) -> usize {
        match self {
            Value::WERT(v) => v.len(),
            Value::TEXT(v) => v.len(),
        }
    }

    pub fn extend_f64(&mut self, value: Vec<f64>) {
        match self {
            Value::WERT(v) => v.extend(value),
            Value::TEXT(_) => {
                warn!("cannot extend f64 to TEXT");
                *self = Self::WERT(value);
            }
        }
    }

    pub fn extend_string(&mut self, value: Vec<String>) {
        match self {
            Value::WERT(_) => {
                warn!("cannot extend string to f64, will renew the value");
                *self = Self::TEXT(value);
            }
            Value::TEXT(v) => v.extend(value),
        }
    }
}

impl From<ValueAttr> for Value {

    fn from(value: ValueAttr) -> Self {
        match value {
            ValueAttr::WERT(v) => Value::WERT(v),
            ValueAttr::TEXT(v) => Value::TEXT(v),
            _ => Value::WERT(vec![]),  // will not come here
        }
    }
}

impl PartialEq for Value {
    
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::WERT(v1), Value::WERT(v2)) => v1 == v2,
            (Value::TEXT(v1), Value::TEXT(v2)) => v1 == v2,
            _ => false,
        }
    }
}

pub struct FESTWERT {
    pub attrs: Vec<StringAttr>,
    pub value: Value,       // for FESTWERT, only one value in the vector
    pub name: String,
}

impl FromStr for FESTWERT {
    type Err=Box<dyn Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut attrs: Vec<StringAttr> = Vec::new();
        let mut lines = s.lines();
        let mut value: Value = Value::new();
        let name = lines.nth(0).unwrap().trim().split_whitespace().last()
                        .ok_or::<&str>("no name found".into())?.to_string();
        for line in lines {
            match line.parse::<Attr>() {
                Ok(Attr::StringAttr(sa)) => attrs.push(sa),
                Ok(Attr::ValueAttr(v)) => value = v.into(),
                Ok(Attr::AxisVar(_)) => {
                    info!("ignoring axis var in FESTWERT");
                }
                Ok(Attr::EmptyLine) => {}
                Err(err_msg) => {
                    warn!("error {} parsing line: {}", err_msg, line);
                }
            }
        }
        if value.len() == 0 {
            return Err(format!("no value found in FESTWERT {name}").into());
        }
        Ok(FESTWERT {
            name,
            attrs,
            value
        })
    }
}


pub struct FESTWERTEBLOCK {
    pub attrs: Vec<StringAttr>,
    pub value: Value,       // for FESTWERT, only one value in the vector
    pub name: String,
    pub dim: usize,
}

impl FromStr for FESTWERTEBLOCK {
    type Err=Box<dyn Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut attrs: Vec<StringAttr> = Vec::new();
        let mut lines = s.lines();
        let mut value: Value = Value::new();
        let mut first_line_words = lines.nth(0).unwrap().trim().split_whitespace();
        let name = first_line_words.nth(1)
                        .ok_or::<&str>("no name found".into())?.to_string();
        let dim = first_line_words.next()
                        .ok_or::<&str>("no dim found".into())?.parse::<usize>()?;
        for line in lines {
            match line.parse::<Attr>() {
                Ok(Attr::StringAttr(sa)) => attrs.push(sa),
                Ok(Attr::ValueAttr(v)) => value = v.into(),
                Ok(Attr::AxisVar(_)) => {
                    info!("ignoring axis var in FESTWERT");
                },
                Ok(Attr::EmptyLine) => {},
                Err(err_msg) => {
                    warn!("error {} parsing line: {}", err_msg, line);
                }
            }
        }
        if value.len() == 0 {
            return Err(format!("no value found in FESTWERT {name}").into());
        }
        Ok(FESTWERTEBLOCK {
            name,
            attrs,
            value,
            dim,
        })
    }
}

pub struct GRUPPENKENNLINIE {
    pub name : String,
    pub attrs: Vec<StringAttr>,
    pub value: Value,
    pub axis: Vec<f64>,
    pub axis_var_name: String,
    pub dim: usize,
}

impl FromStr for GRUPPENKENNLINIE {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut lines = s.lines();
        let mut attrs = Vec::new();
        let mut value = Value::new();
        let mut axis = Vec::new();
        let mut axis_var_name = String::from("no_axis_var_name_found");
        let first_line = lines.next().ok_or("no first line found in GRUPPENKENNLINIE")?;
        let name = first_line.trim().split_whitespace().nth(1).ok_or("no name found")?.to_string();
        let dim = first_line.trim().split_whitespace().nth(2).ok_or("no dim found")?.parse::<usize>().unwrap();
        for line in lines {
            match line.parse::<Attr>() {
                Ok(Attr::StringAttr(sa)) => attrs.push(sa),
                Ok(Attr::ValueAttr(va)) => {
                    if let ValueAttr::WERT(w) = va{
                        value.extend_f64(w);
                    } else if let ValueAttr::STX(sx) = va {
                        axis.extend(sx.into_iter());
                    } else if let ValueAttr::TEXT(t) = va {
                        value = Value::TEXT(t);
                        info!("found text value: {:?}", value);
                    } else {
                        return Err("unknown value type".into());
                    }
                },
                Ok(Attr::AxisVar(av)) => {
                    if av.axistype == AxisType::X {
                        axis_var_name = av.identifier;
                    }
                },
                Ok(Attr::EmptyLine) => {},
                Err(error_msg) => {
                    warn!("error parsing line: {}, error: {}", line, error_msg);  //shall not stop the parser
                }
            }
        }
        Ok(GRUPPENKENNLINIE {
            name,
            attrs,
            value,
            axis,
            axis_var_name,
            dim
        }
        )
    }
}

pub struct STUETZSTELLENVERTEILUNG {
    pub name: String,
    pub attrs: Vec<StringAttr>,
    pub value: Vec<f64>,
    pub dim: usize,
}

impl FromStr for STUETZSTELLENVERTEILUNG {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut lines = s.lines();
        let mut attrs = Vec::new();
        let mut value: Vec<f64> = Vec::new();
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
                        value.extend(w.into_iter());
                    } else {
                        return Err("unknown value type".into());
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

pub struct GRUPPENKENNFELD {
    pub name: String,
    pub attrs: Vec<StringAttr>,
    pub value: Vec<Value>,
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
        })
    }
}


pub enum Block {
    Constant(FESTWERT),
    ConstantBlock(FESTWERTEBLOCK),
    Table(GRUPPENKENNLINIE),
    Distribution(STUETZSTELLENVERTEILUNG),
    Map(GRUPPENKENNFELD),
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;


    #[rstest]
    fn test_festwert() {
        let festwert = r#"FESTWERT CDCBlnd_RatDmpgBmpPthlLimEna_C 
                                LANGNAME "Damping ratio limitation for Bump/Pothole enable/disable" 
                                EINHEIT_W "unitless"
                                WERT 1.0000000000000000
                                END"#;
        let f: FESTWERT = festwert.parse().unwrap();
        assert_eq!(f.name, "CDCBlnd_RatDmpgBmpPthlLimEna_C");
        assert_eq!(f.value.len(), 1);
        if let Value::WERT(v) = f.value {
            assert_eq!(v[0], 1.0);
        }
        assert_eq!(f.attrs.len(), 2);
    }

    #[rstest]
    fn test_festwert_text() {
        let festwert = r#"FESTWERT SusApp_stHeiReqMan_c 
                                LANGNAME "Manual Override: Height Leveling Request for rear spring" 
                                EINHEIT_W "na"
                                TEXT "SprgLvlEnum_None"
                                END"#;
        let f: FESTWERT = festwert.parse().unwrap();
        assert_eq!(f.name, "SusApp_stHeiReqMan_c");
        assert_eq!(f.value.len(), 1);
        if let Value::TEXT(v) = f.value {
            assert_eq!(v[0], "SprgLvlEnum_None");
        }
        assert_eq!(f.attrs.len(), 2);
        assert_eq!(eval_string_attr(&f.attrs, "EINHEIT_W").unwrap(), "na");
    }

    #[rstest]
    fn test_one_dim_table() {
        let dcm_line = r#"GRUPPENKENNLINIE CDCBlnd_ModeSelCor_T 3
                                LANGNAME "Correction map selection for pitch, heave & roll rate based on selected damping mode" 
                                EINHEIT_X "unitless"
                                EINHEIT_W "unitless"
                                *SSTX	CDCBlnd_ModeSel_Ax
                                ST/X   1.0000000000000000   2.0000000000000000   3.0000000000000000   
                                WERT   1.0000000000000000   2.0000000000000000   3.0000000000000000   
                                END"#;
        let table:GRUPPENKENNLINIE = dcm_line.parse().unwrap();
        assert_eq!(table.name, "CDCBlnd_ModeSelCor_T");
        assert_eq!(table.dim, 3);
        assert_eq!(table.value, Value::WERT(vec![1.0, 2.0, 3.0]));
        assert_eq!(table.axis, vec![1.0, 2.0, 3.0]);
        assert_eq!(table.axis_var_name, "CDCBlnd_ModeSel_Ax");
        assert_eq!(table.attrs.len(), 3);
    }

    #[rstest]
    fn test_one_dim_table_ex () {
        let dcm_line = r#"GRUPPENKENNLINIE CDCAct_DmprICmdRear_T 9
                                LANGNAME "Damping ratio to damper current conversion for rear dampers" 
                                EINHEIT_X "percent"
                                EINHEIT_W "mA"
                                *SSTX	CDCAct_RatDmpg_Ax
                                ST/X   0.0000000000000000   12.5000000000000000   25.0000000000000000   37.5000000000000000   50.0000000000000000   62.5000000000000000   
                                ST/X   75.0000000000000000   87.5000000000000000   100.0000000000000000   
                                WERT   320.0000000000000000   480.0000000000000000   640.0000000000000000   800.0000000000000000   960.0000000000000000   1120.0000000000000000   
                                WERT   1280.0000000000000000   1440.0000000000000000   1600.0000000000000000   
                                END
                                "#;

        let table = dcm_line.parse::<GRUPPENKENNLINIE>().unwrap();
        assert_eq!(table.name, "CDCAct_DmprICmdRear_T");
        assert_eq!(table.dim, 9);
        assert_eq!(table.value.len(), 9);
        assert_eq!(table.axis.len(), 9);
        assert_eq!(table.value, Value::WERT(vec![320.0, 480.0, 640.0, 800.0, 960.0, 1120.0, 1280.0, 1440.0, 1600.0]));
        assert_eq!(table.axis, vec![0.0, 12.5, 25.0, 37.5, 50.0, 62.5, 75.0, 87.5, 100.0]);
    }

    #[rstest]
    fn test_axis_points() {
        let dcm_block = r#"STUETZSTELLENVERTEILUNG CDCBlnd_RatDmpgLimVehSpd_Ax 9
                                *SST
                                LANGNAME "CDCBlnd damping ratio limitation vehicle speed breakpoint" 
                                EINHEIT_X "km/h"
                                ST/X   0.0000000000000000   29.9812488555908200   39.9937477111816410   60.0187492370605470   79.9874954223632810   100.0124969482421900   
                                ST/X   119.9812469482421900   150.0187530517578100   200.0249938964843700   
                                END"#;
        let table = dcm_block.parse::<STUETZSTELLENVERTEILUNG>().unwrap();
        assert_eq!(table.name, "CDCBlnd_RatDmpgLimVehSpd_Ax");
        assert_eq!(table.dim, 9);
        assert_eq!(table.value.len(), 9);
        assert_eq!(table.attrs.len(), 2);
        assert_eq!(table.value, vec![0.0, 29.9812488555908200, 39.9937477111816410, 60.0187492370605470, 79.9874954223632810, 100.0124969482421900, 119.9812469482421900, 150.0187530517578100, 200.0249938964843700]);      
    }

    #[rstest]
    fn test_parse_2d_map() {
        let dcm_block = r#"GRUPPENKENNFELD CDCBlnd_RatDmpgMaxFrnt_M 3 7
                                LANGNAME "Maximum damping ratio based on damper mode and vehicle speed" 
                                EINHEIT_X "unitless"
                                EINHEIT_Y "km/h"
                                EINHEIT_W "percent"
                                *SSTX	CDCBlnd_ModeSel_Ax
                                *SSTY	CDCBlnd_RatDmpgLimVehSpd_Ax
                                ST/X   1.0000000000000000   2.0000000000000000   3.0000000000000000   
                                ST/Y   0.0000000000000000
                                WERT   100.0000000000000000   100.0000000000000000   100.0000000000000000   
                                ST/Y   29.9812488555908200
                                WERT   100.0000000000000000   100.0000000000000000   100.0000000000000000   
                                ST/Y   39.9937477111816410
                                WERT   100.0000000000000000   100.0000000000000000   100.0000000000000000   
                                ST/Y   60.0187492370605470
                                WERT   100.0000000000000000   100.0000000000000000   100.0000000000000000   
                                ST/Y   79.9874954223632810
                                WERT   100.0000000000000000   100.0000000000000000   100.0000000000000000   
                                ST/Y   100.0124969482421900
                                WERT   100.0000000000000000   100.0000000000000000   100.0000000000000000   
                                ST/Y   119.9812469482421900
                                WERT   100.0000000000000000   100.0000000000000000   100.000000000000
                                "#;
        let block = dcm_block.parse::<GRUPPENKENNFELD>().unwrap();
        assert_eq!(block.name, "CDCBlnd_RatDmpgMaxFrnt_M");
        assert_eq!(block.dim, (3, 7));
        assert_eq!(block.x_axis, vec![1., 2., 3.]);
        assert_eq!(block.y_axis, vec![0.0, 29.9812488555908200, 39.9937477111816410, 60.0187492370605470, 79.9874954223632810, 100.0124969482421900, 119.9812469482421900]);
        assert_eq!(block.value.len(), 7);
        assert_eq!(block.value[0], Value::WERT(vec![100.0, 100.0, 100.0]));
        assert_eq!(block.value[1], Value::WERT(vec![100.0, 100.0, 100.0]));
        assert_eq!(block.value[2], Value::WERT(vec![100.0, 100.0, 100.0]));
        assert_eq!(block.value[3], Value::WERT(vec![100.0, 100.0, 100.0]));
        assert_eq!(block.value[4], Value::WERT(vec![100.0, 100.0, 100.0]));
        assert_eq!(block.value[5], Value::WERT(vec![100.0, 100.0, 100.0]));
        assert_eq!(block.value[6], Value::WERT(vec![100.0, 100.0, 100.0]));
    }

    #[rstest]
    fn test_parse_map2() {
        let dcm_block = r#"GRUPPENKENNFELD CDCRdPMC_RollCtrlFac1CmprsFrnt_M 16 9
                                LANGNAME "Roll factor map based on vehicle speed and body roll rate for selected damping mode at compression state for front axle" 
                                EINHEIT_X "deg/s"
                                EINHEIT_Y "km/h"
                                EINHEIT_W "unitless"
                                *SSTX	CDCRdPMC_BdyRollRate_Ax
                                *SSTY	CDCRdPMC_CtrlCorFacVehSpd_Ax
                                ST/X   -21.0000000000000000   -15.0000000000000000   -9.0000000000000000   -6.0000000000000000   -3.0000000000000000   -2.0000000000000000   
                                ST/X   -1.0000000000000000   -0.5000000000000000   0.5000000000000000   1.0000000000000000   2.0000000000000000   3.0000000000000000   
                                ST/X   6.0000000000000000   9.0000000000000000   15.0000000000000000   21.0000000000000000   
                                ST/Y   0.0000000000000000
                                WERT   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   
                                WERT   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   
                                WERT   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   
                                ST/Y   20.0249996185302730
                                WERT   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   
                                WERT   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   
                                WERT   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   
                                ST/Y   39.9937477111816410
                                WERT   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   
                                WERT   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   
                                WERT   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   
                                ST/Y   60.0187492370605470
                                WERT   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   
                                WERT   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   
                                WERT   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   
                                ST/Y   79.9874954223632810
                                WERT   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   
                                WERT   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   
                                WERT   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   
                                ST/Y   100.0124969482421900
                                WERT   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   
                                WERT   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   
                                WERT   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   
                                ST/Y   119.9812469482421900
                                WERT   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   
                                WERT   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   
                                WERT   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   
                                ST/Y   150.0187530517578100
                                WERT   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   
                                WERT   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   
                                WERT   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   
                                ST/Y   209.9812469482421900
                                WERT   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   
                                WERT   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   
                                WERT   0.0000000000000000   0.0000000000000000   0.0000000000000000   0.0000000000000000   
                                END"#;
        let map: GRUPPENKENNFELD = dcm_block.parse().unwrap();
        assert_eq!(map.name, "CDCRdPMC_RollCtrlFac1CmprsFrnt_M");
        assert_eq!(map.dim, (16, 9));
        assert_eq!(map.x_axis_name, "CDCRdPMC_BdyRollRate_Ax");
        assert_eq!(map.y_axis_name, "CDCRdPMC_CtrlCorFacVehSpd_Ax");
        assert_eq!(map.x_axis, vec![-21.0, -15.0, -9.0, -6.0, -3.0, -2.0, -1.0, -0.5, 0.5, 1.0, 2.0, 3.0, 6.0, 9.0, 15.0, 21.0]);
        assert_eq!(map.y_axis, vec![0.0, 20.0249996185302730, 39.9937477111816410, 60.0187492370605470, 79.9874954223632810, 100.0124969482421900, 119.9812469482421900, 150.0187530517578100, 209.9812469482421900]);
        assert_eq!(eval_string_attr(&map.attrs, "LANGNAME").unwrap(), "Roll factor map based on vehicle speed and body roll rate for selected damping mode at compression state for front axle");
        assert_eq!(eval_string_attr(&map.attrs, "EINHEIT_X").unwrap(), "deg/s");
        assert_eq!(eval_string_attr(&map.attrs, "EINHEIT_Y").unwrap(), "km/h");
        assert_eq!(eval_string_attr(&map.attrs, "EINHEIT_W").unwrap(), "unitless");
    }

    #[rstest]
    fn test_festwert_block() {
        let dcm_block = r#"FESTWERTEBLOCK SLC_LC_CP_flgDoorChkDiBootLowr_c 6
                                LANGNAME "Door check setup for Easy Entry Control, 1 = Disabled 0 = Enabled [FL,FR,RL,RR,Boot,Bonnet]" 
                                EINHEIT_W "na"
                                WERT   1.0000000000000000   1.0000000000000000   1.0000000000000000   1.0000000000000000   0.0000000000000000   1.0000000000000000   
                                END
                                "#;
        let block = dcm_block.parse::<FESTWERTEBLOCK>().unwrap();
        assert_eq!(block.name, "SLC_LC_CP_flgDoorChkDiBootLowr_c");
        assert_eq!(block.value, Value::WERT(vec![1.0, 1.0, 1.0, 1.0, 0.0, 1.0]));
        assert_eq!(block.dim, 6);
    }
}

