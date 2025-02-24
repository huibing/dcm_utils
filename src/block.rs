use crate::attr::string_attr::{StringAttr, is_string_attr, AxisVar};
use crate::attr::value_attr::{ValueAttr, is_value_attr_line};
use std::str::FromStr;
use std::error::Error;

/*
FESTWERT CDCBlnd_RatDmpgBmpPthlLimEna_C 
   LANGNAME "Damping ratio limitation for Bump/Pothole enable/disable" 
   EINHEIT_W "unitless"
   WERT 1.0000000000000000
END 

GRUPPENKENNLINIE CDCBlnd_ModeSelCor_T 3
   LANGNAME "Correction map selection for pitch, heave & roll rate based on selected damping mode" 
   EINHEIT_X "unitless"
   EINHEIT_W "unitless"
*SSTX	CDCBlnd_ModeSel_Ax
   ST/X   1.0000000000000000   2.0000000000000000   3.0000000000000000   
   WERT   1.0000000000000000   2.0000000000000000   3.0000000000000000   
END

STUETZSTELLENVERTEILUNG CDCBlnd_RatDmpgLimVehSpd_Ax 9
*SST
   LANGNAME "CDCBlnd damping ratio limitation vehicle speed breakpoint" 
   EINHEIT_X "km/h"
   ST/X   0.0000000000000000   29.9812488555908200   39.9937477111816410   60.0187492370605470   79.9874954223632810   100.0124969482421900   
   ST/X   119.9812469482421900   150.0187530517578100   200.0249938964843700   
END

GRUPPENKENNFELD CDCBlnd_RatDmpgMaxFrnt_M 3 9
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
   WERT   100.0000000000000000   100.0000000000000000   100.0000000000000000   
   ST/Y   150.0187530517578100
   WERT   100.0000000000000000   100.0000000000000000   100.0000000000000000   
   ST/Y   200.0249938964843700
   WERT   100.0000000000000000   100.0000000000000000   100.0000000000000000   
END
*/

pub struct FESTWERT {
    pub attrs: Vec<StringAttr>,
    pub value: Vec<f64>,
    pub name: String,
}

impl FromStr for FESTWERT {
    type Err=Box<dyn Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut attrs: Vec<StringAttr> = Vec::new();
        let mut lines = s.lines();
        let mut value= Vec::new();
        let name = lines.nth(0).unwrap().trim().split_whitespace().last()
                        .ok_or::<&str>("no name found".into())?.to_string();
        for line in lines {
            if is_string_attr(line) {
                attrs.push(line.parse()?);
            } else if is_value_attr_line(line) {
                let value_attr: ValueAttr = line.trim().parse()?;
                value = value_attr.into();
            }
        }
        Ok(FESTWERT {
            name,
            attrs,
            value
        })
    }
}

pub struct GRUPPENKENNLINIE {
    pub name : String,
    pub attrs: Vec<StringAttr>,
    pub value: Vec<f64>,
    pub axis: Vec<f64>,
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
        assert_eq!(f.value[0], 1.0);
        assert_eq!(f.attrs.len(), 2);
    }
}

