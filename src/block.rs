use crate::blocks::{
    FESTWERT, FESTWERTEBLOCK, GRUPPENKENNFELD, GRUPPENKENNLINIE, STUETZSTELLENVERTEILUNG,
};
use crate::value::f32_bytes_eq_slice;
use crate::value::Value;

#[derive(Clone, Debug)]
pub enum Block {
    Constant(FESTWERT),
    ConstantBlock(FESTWERTEBLOCK),
    Table(GRUPPENKENNLINIE),
    Distribution(STUETZSTELLENVERTEILUNG),
    Map(GRUPPENKENNFELD),
}

impl Block {
    pub fn get_values(&self) -> &Value {
        match self {
            Block::Constant(c) => &c.value,
            Block::ConstantBlock(c) => &c.value,
            Block::Table(t) => &t.value,
            Block::Distribution(d) => &d.value,
            Block::Map(m) => &m.value_flat,
        }
    }

    pub fn get_name(&self) -> &str {
        match self {
            Block::Constant(c) => &c.name,
            Block::ConstantBlock(c) => &c.name,
            Block::Table(t) => &t.name,
            Block::Distribution(d) => &d.name,
            Block::Map(m) => &m.name,
        }
    }

    pub fn get_attr(&self, attr_name: &str) -> Option<String> {
        let attrs = match self {
            Block::Constant(c) => &c.attrs,
            Block::ConstantBlock(c) => &c.attrs,
            Block::Table(t) => &t.attrs,
            Block::Distribution(d) => &d.attrs,
            Block::Map(m) => &m.attrs,
        };
        attrs
            .iter()
            .find(|k| k.identifier == attr_name)
            .map(|v| v.value.clone())
    }

    pub fn get_w_unit(&self) -> Option<String> {
        self.get_attr("EINHEIT_W")
    }

    pub fn get_x_unit(&self) -> Option<String> {
        self.get_attr("EINHEIT_X")
    }

    pub fn get_y_unit(&self) -> Option<String> {
        self.get_attr("EINHEIT_Y")
    }

    pub fn get_desc(&self) -> Option<String> {
        self.get_attr("LANGNAME")
    }

    pub fn get_x_var_name(&self) -> Option<String> {
        match self {
            Block::Map(m) => Some(m.x_axis_name.clone()),
            Block::Table(t) => Some(t.axis_var_name.clone()),
            _ => None,
        }
    }

    pub fn get_y_var_name(&self) -> Option<String> {
        match self {
            Block::Map(m) => Some(m.y_axis_name.clone()),
            _ => None,
        }
    }

    pub fn f32_bytes_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Block::Constant(c1), Block::Constant(c2)) => c1.value.f32_bytes_eq(&c2.value),
            (Block::ConstantBlock(b1), Block::ConstantBlock(b2)) => {
                b1.value.f32_bytes_eq(&b2.value)
            }
            (Block::Table(t1), Block::Table(t2)) => {
                t1.value.f32_bytes_eq(&t2.value)
                    && f32_bytes_eq_slice(&t1.axis, &t2.axis)
                    && t1.axis_var_name == t2.axis_var_name
            }
            (Block::Distribution(d1), Block::Distribution(d2)) => d1.value.f32_bytes_eq(&d2.value),
            (Block::Map(m1), Block::Map(m2)) => {
                m1.value_flat.f32_bytes_eq(&m2.value_flat)
                    && f32_bytes_eq_slice(&m1.x_axis, &m2.x_axis)
                    && f32_bytes_eq_slice(&m1.y_axis, &m2.y_axis)
                    && m1.x_axis_name == m2.x_axis_name
                    && m1.y_axis_name == m2.y_axis_name
            }
            _ => false,
        }
    }
}

impl PartialEq for Block {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Block::Constant(c1), Block::Constant(c2)) => c1.value == c2.value,
            (Block::ConstantBlock(b1), Block::ConstantBlock(b2)) => b1.value == b2.value,
            (Block::Table(t1), Block::Table(t2)) => {
                t1.value == t2.value && t1.axis == t2.axis && t1.axis_var_name == t2.axis_var_name
            }
            (Block::Distribution(d1), Block::Distribution(d2)) => d1.value == d2.value,
            (Block::Map(m1), Block::Map(m2)) => {
                m1.value_flat == m2.value_flat
                    && m1.x_axis == m2.x_axis
                    && m1.y_axis == m2.y_axis
                    && m1.x_axis_name == m2.x_axis_name
                    && m1.y_axis_name == m2.y_axis_name
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attr::string_attr::eval_string_attr;
    use rstest::*;

    #[rstest]
    fn test_block_partial_eq_map_checks_axis_names() {
        let map1 = GRUPPENKENNFELD::from_f64(
            "test_map",
            vec![vec![1.0, 2.0], vec![3.0, 4.0]],
            vec![1.0, 2.0],
            vec![0.0, 1.0],
            "axis_x_v1",
            "axis_y_v1",
            "desc",
            "unit_w",
            "unit_x",
            "unit_y",
        );
        let map2 = GRUPPENKENNFELD::from_f64(
            "test_map",
            vec![vec![1.0, 2.0], vec![3.0, 4.0]],
            vec![1.0, 2.0],
            vec![0.0, 1.0],
            "axis_x_v1",
            "axis_y_v2",
            "desc",
            "unit_w",
            "unit_x",
            "unit_y",
        );
        let b1 = Block::Map(map1);
        let b2 = Block::Map(map2);
        assert_ne!(
            b1, b2,
            "Block::PartialEq should detect different y_axis_name"
        );
    }

    #[rstest]
    fn test_block_f32_bytes_eq_constant() {
        let c1 = FESTWERT::from_f64(
            "test".to_string(),
            1.0,
            "desc".to_string(),
            "unit".to_string(),
        );
        let c2 = FESTWERT::from_f64(
            "test".to_string(),
            1.0 + 1e-9,
            "desc".to_string(),
            "unit".to_string(),
        );
        let b1 = Block::Constant(c1);
        let b2 = Block::Constant(c2);
        assert!(b1.f32_bytes_eq(&b2));
        assert_ne!(b1, b2);
    }

    #[rstest]
    fn test_block_f32_bytes_eq_constant_block() {
        let cb1 = FESTWERTEBLOCK::from_f64(
            "test_cb".to_string(),
            vec![1.0, 2.0, 3.0],
            "desc".to_string(),
            "unit".to_string(),
        );
        let cb2 = FESTWERTEBLOCK::from_f64(
            "test_cb".to_string(),
            vec![1.0 + 1e-9, 2.0, 3.0],
            "desc".to_string(),
            "unit".to_string(),
        );
        let b1 = Block::ConstantBlock(cb1);
        let b2 = Block::ConstantBlock(cb2);
        assert!(b1.f32_bytes_eq(&b2));
        assert_ne!(b1, b2);
    }

    #[rstest]
    fn test_block_f32_bytes_eq_table() {
        let t1 = GRUPPENKENNLINIE::from_f64(
            "test_tbl",
            &[1.0, 2.0, 3.0],
            "desc",
            "unit_w",
            "unit_x",
            "axis_x",
            &[0.0, 10.0, 20.0],
        );
        let t2 = GRUPPENKENNLINIE::from_f64(
            "test_tbl",
            &[1.0 + 1e-9, 2.0, 3.0],
            "desc",
            "unit_w",
            "unit_x",
            "axis_x",
            &[0.0, 10.0 + 1e-9, 20.0],
        );
        let b1 = Block::Table(t1);
        let b2 = Block::Table(t2);
        assert!(b1.f32_bytes_eq(&b2));
        assert_ne!(b1, b2);
    }

    #[rstest]
    fn test_block_f32_bytes_eq_distribution() {
        let d1 = STUETZSTELLENVERTEILUNG::from_f64("test_dist", "desc", &[0.0, 10.0, 20.0], "unit");
        let d2 = STUETZSTELLENVERTEILUNG::from_f64(
            "test_dist",
            "desc",
            &[0.0, 10.0 + 1e-9, 20.0],
            "unit",
        );
        let b1 = Block::Distribution(d1);
        let b2 = Block::Distribution(d2);
        assert!(b1.f32_bytes_eq(&b2));
        assert_ne!(b1, b2);
    }

    #[rstest]
    fn test_block_f32_bytes_eq_map() {
        let map1 = GRUPPENKENNFELD::from_f64(
            "test_map",
            vec![vec![1.0, 2.0], vec![3.0, 4.0]],
            vec![1.0, 2.0],
            vec![0.0, 1.0],
            "axis_x",
            "axis_y",
            "desc",
            "unit_w",
            "unit_x",
            "unit_y",
        );
        let map2 = GRUPPENKENNFELD::from_f64(
            "test_map",
            vec![vec![1.0 + 1e-9, 2.0], vec![3.0, 4.0]],
            vec![1.0, 2.0],
            vec![0.0, 1.0],
            "axis_x",
            "axis_y",
            "desc",
            "unit_w",
            "unit_x",
            "unit_y",
        );
        let b1 = Block::Map(map1);
        let b2 = Block::Map(map2);
        assert!(b1.f32_bytes_eq(&b2));
        assert_ne!(b1, b2);
    }

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
        let table: GRUPPENKENNLINIE = dcm_line.parse().unwrap();
        assert_eq!(table.name, "CDCBlnd_ModeSelCor_T");
        assert_eq!(table.dim, 3);
        assert_eq!(table.value, Value::WERT(vec![1.0, 2.0, 3.0]));
        assert_eq!(table.axis, vec![1.0, 2.0, 3.0]);
        assert_eq!(table.axis_var_name, "CDCBlnd_ModeSel_Ax");
        assert_eq!(table.attrs.len(), 3);
    }

    #[rstest]
    fn test_one_dim_table_ex() {
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
        assert_eq!(
            table.value,
            Value::WERT(vec![
                320.0, 480.0, 640.0, 800.0, 960.0, 1120.0, 1280.0, 1440.0, 1600.0
            ])
        );
        assert_eq!(
            table.axis,
            vec![0.0, 12.5, 25.0, 37.5, 50.0, 62.5, 75.0, 87.5, 100.0]
        );
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
        assert_eq!(
            table.value,
            Value::WERT(vec![
                0.0,
                29.9812488555908200,
                39.9937477111816410,
                60.0187492370605470,
                79.9874954223632810,
                100.0124969482421900,
                119.9812469482421900,
                150.0187530517578100,
                200.0249938964843700
            ])
        );
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
        assert_eq!(
            block.y_axis,
            vec![
                0.0,
                29.9812488555908200,
                39.9937477111816410,
                60.0187492370605470,
                79.9874954223632810,
                100.0124969482421900,
                119.9812469482421900
            ]
        );
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
        assert_eq!(
            map.x_axis,
            vec![
                -21.0, -15.0, -9.0, -6.0, -3.0, -2.0, -1.0, -0.5, 0.5, 1.0, 2.0, 3.0, 6.0, 9.0,
                15.0, 21.0
            ]
        );
        assert_eq!(
            map.y_axis,
            vec![
                0.0,
                20.0249996185302730,
                39.9937477111816410,
                60.0187492370605470,
                79.9874954223632810,
                100.0124969482421900,
                119.9812469482421900,
                150.0187530517578100,
                209.9812469482421900
            ]
        );
        assert_eq!(eval_string_attr(&map.attrs, "LANGNAME").unwrap(), "Roll factor map based on vehicle speed and body roll rate for selected damping mode at compression state for front axle");
        assert_eq!(eval_string_attr(&map.attrs, "EINHEIT_X").unwrap(), "deg/s");
        assert_eq!(eval_string_attr(&map.attrs, "EINHEIT_Y").unwrap(), "km/h");
        assert_eq!(
            eval_string_attr(&map.attrs, "EINHEIT_W").unwrap(),
            "unitless"
        );
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
        let blk = Block::ConstantBlock(block);
        assert_eq!(blk.get_name(), "SLC_LC_CP_flgDoorChkDiBootLowr_c");
        assert_eq!(blk.get_w_unit().unwrap(), "na");
        assert_eq!(blk.get_desc().unwrap(), "Door check setup for Easy Entry Control, 1 = Disabled 0 = Enabled [FL,FR,RL,RR,Boot,Bonnet]");
    }
}
