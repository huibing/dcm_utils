use std::collections::HashMap;
use std::error::Error;
use std::ffi::OsString;
use std::path::Path;

use a2ldeser::extractor::{ExtractedObject, Extractor, PhysicalValue};
use a2ldeser::hex_reader::HexMemory;
use a2lfile::A2lObjectName;

use crate::DcmData;
use crate::block::Block;
use crate::blocks::{FESTWERT, FESTWERTEBLOCK, GRUPPENKENNLINIE, GRUPPENKENNFELD, STUETZSTELLENVERTEILUNG};
use indexmap::IndexMap;

/// Parse A2L+HEX, extract all calibration characteristics, and return DcmData
pub fn gen_dcm_data(a2l: &Path, hex: &Path) -> Result<DcmData, Box<dyn Error>> {
    let a2l_path = OsString::from(a2l.as_os_str());
    let (a2l_obj, _warnings) = a2lfile::load(a2l_path, None, false)
        .map_err(|e| format!("Failed to parse A2L file '{}': {}", a2l.display(), e))?;
    let module = &a2l_obj.project.module[0];

    let hex_mem = HexMemory::from_file(hex)
        .map_err(|e| format!("Failed to parse HEX file '{}': {}", hex.display(), e))?;

    let extractor = Extractor::new(module, &hex_mem);
    let report = extractor.extract_all();

    for fail in &report.failures {
        eprintln!("SKIP: {} - {:?}", fail.name, fail.error);
    }
    eprintln!(
        "Extracted {}/{} characteristics",
        report.successes.len(),
        report.total()
    );

    let blocks = extracted_to_dcm_blocks(report.successes, module);

    Ok(DcmData::from_blocks(blocks))
}

fn extracted_to_dcm_blocks(
    objects: Vec<ExtractedObject>,
    module: &a2lfile::Module,
) -> IndexMap<String, Block> {
    let mut blocks = IndexMap::new();

    // Build long_identifier lookup map from A2L characteristics
    let mut langname_map: HashMap<String, String> = HashMap::new();
    for chr in &module.characteristic {
        langname_map.insert(chr.get_name().to_string(), chr.long_identifier.clone());
    }
    for apt in &module.axis_pts {
        langname_map.entry(apt.get_name().to_string())
            .or_insert_with(|| apt.long_identifier.clone());
    }

    let get_langname = |name: &str| -> String {
        langname_map.get(name).cloned().unwrap_or_else(|| name.to_string())
    };

    // Partition objects by type
    let mut values = Vec::new();
    let mut val_blks = Vec::new();
    let mut asciis = Vec::new();
    let mut curves = Vec::new();
    let mut maps = Vec::new();
    let mut axis_pts_list = Vec::new();

    for obj in objects {
        match obj {
            ExtractedObject::Value(v) => values.push(v),
            ExtractedObject::ValBlk(v) => val_blks.push(v),
            ExtractedObject::Ascii(a) => asciis.push(a),
            ExtractedObject::Curve(c) => curves.push(c),
            ExtractedObject::Map(m) => maps.push(m),
            ExtractedObject::AxisPts(a) => axis_pts_list.push(a),
        }
    }

    // Group 1: FESTWERT (Value)
    for v in values {
        let langname = get_langname(&v.name);
        let block = extracted_value_to_block(&v, &langname);
        blocks.insert(v.name.clone(), block);
    }

    // Group 2: FESTWERT (Ascii)
    for a in asciis {
        let langname = get_langname(&a.name);
        let block = Block::Constant(FESTWERT::from_string(
            a.name.clone(), a.text, langname, String::new(),
        ));
        blocks.insert(a.name.clone(), block);
    }

    // Group 3: FESTWERTEBLOCK
    for vb in val_blks {
        let langname = get_langname(&vb.name);
        let block = extracted_valblk_to_block(&vb, &langname);
        blocks.insert(vb.name.clone(), block);
    }

    // Group 4: STUETZSTELLENVERTEILUNG (standalone + derived)
    for apt in &axis_pts_list {
        let langname = get_langname(&apt.name);
        let block = Block::Distribution(STUETZSTELLENVERTEILUNG::from_f64(
            &apt.name, &langname, &apt.values, &apt.unit,
        ));
        blocks.insert(apt.name.clone(), block);
    }
    for c in &curves {
        let axis_name = format!("{}_X", c.name);
        if !blocks.contains_key(&axis_name) {
            let langname = get_langname(&c.name);
            let block = Block::Distribution(STUETZSTELLENVERTEILUNG::from_f64(
                &axis_name, &langname, &c.x_axis, &c.x_unit,
            ));
            blocks.insert(axis_name, block);
        } else {
            eprintln!("WARN: Derived axis '{}' collides with existing AXIS_PTS, skipped", axis_name);
        }
    }
    for m in &maps {
        let x_axis_name = format!("{}_X", m.name);
        let y_axis_name = format!("{}_Y", m.name);
        for (axis_name, axis_values, axis_unit) in [
            (&x_axis_name, &m.x_axis, &m.x_unit),
            (&y_axis_name, &m.y_axis, &m.y_unit),
        ] {
            if !blocks.contains_key(axis_name.as_str()) {
                let langname = get_langname(&m.name);
                let block = Block::Distribution(STUETZSTELLENVERTEILUNG::from_f64(
                    axis_name, &langname, axis_values, axis_unit,
                ));
                blocks.insert(axis_name.clone(), block);
            } else {
                eprintln!("WARN: Derived axis '{}' collides with existing AXIS_PTS, skipped", axis_name);
            }
        }
    }

    // Group 5: GRUPPENKENNLINIE (skip if contains verbal values)
    for c in curves {
        let langname = get_langname(&c.name);
        if let Some(block) = extracted_curve_to_block(&c, &langname) {
            blocks.insert(c.name.clone(), block);
        }
    }

    // Group 6: GRUPPENKENNFELD (skip if contains verbal values)
    for m in maps {
        let langname = get_langname(&m.name);
        if let Some(block) = extracted_map_to_block(&m, &langname) {
            blocks.insert(m.name.clone(), block);
        }
    }

    blocks
}

fn extracted_value_to_block(v: &a2ldeser::extractor::ExtractedValue, langname: &str) -> Block {
    let unit = v.unit.clone();
    match &v.physical {
        PhysicalValue::Numeric(n) => {
            Block::Constant(FESTWERT::from_f64(v.name.clone(), *n, langname.to_string(), unit))
        }
        PhysicalValue::Verbal(s) => {
            Block::Constant(FESTWERT::from_string(v.name.clone(), s.clone(), langname.to_string(), unit))
        }
    }
}

fn extracted_valblk_to_block(vb: &a2ldeser::extractor::ExtractedValBlk, langname: &str) -> Block {
    let unit = vb.unit.clone();
    let has_verbal = vb.values.iter().any(|pv| matches!(pv, PhysicalValue::Verbal(_)));

    if has_verbal {
        let strings: Vec<String> = vb.values.iter().map(|pv| match pv {
            PhysicalValue::Numeric(n) => format!("{}", n),
            PhysicalValue::Verbal(s) => s.clone(),
        }).collect();
        Block::ConstantBlock(FESTWERTEBLOCK::from_string(
            vb.name.clone(), strings, langname.to_string(), unit,
        ))
    } else {
        let nums: Vec<f64> = vb.values.iter().map(|pv| match pv {
            PhysicalValue::Numeric(n) => *n,
            _ => unreachable!(),
        }).collect();
        Block::ConstantBlock(FESTWERTEBLOCK::from_f64(
            vb.name.clone(), nums, langname.to_string(), unit,
        ))
    }
}

fn extracted_curve_to_block(c: &a2ldeser::extractor::ExtractedCurve, langname: &str) -> Option<Block> {
    let axis_name = format!("{}_X", c.name);
    let unit = c.unit.clone();
    let unit_x = c.x_unit.clone();

    let has_verbal = c.values.iter().any(|pv| matches!(pv, PhysicalValue::Verbal(_)));
    if has_verbal {
        eprintln!("WARN: Skipping CURVE '{}' — contains verbal values, not representable in DCM", c.name);
        return None;
    }

    let nums: Vec<f64> = c.values.iter().map(|pv| match pv {
        PhysicalValue::Numeric(n) => *n,
        _ => unreachable!(),
    }).collect();

    Some(Block::Table(GRUPPENKENNLINIE::from_f64(
        &c.name,
        &nums,
        langname,
        &unit,
        &unit_x,
        &axis_name,
        &c.x_axis,
    )))
}

fn extracted_map_to_block(m: &a2ldeser::extractor::ExtractedMap, langname: &str) -> Option<Block> {
    let x_axis_name = format!("{}_X", m.name);
    let y_axis_name = format!("{}_Y", m.name);
    let unit_w = m.unit.clone();
    let unit_x = m.x_unit.clone();
    let unit_y = m.y_unit.clone();

    let has_verbal = m.values.iter().any(|row| {
        row.iter().any(|pv| matches!(pv, PhysicalValue::Verbal(_)))
    });
    if has_verbal {
        eprintln!("WARN: Skipping MAP '{}' — contains verbal values, not representable in DCM", m.name);
        return None;
    }

    let values_2d: Vec<Vec<f64>> = m.values.iter().map(|row| {
        row.iter().map(|pv| match pv {
            PhysicalValue::Numeric(n) => *n,
            _ => unreachable!(),
        }).collect()
    }).collect();

    Some(Block::Map(GRUPPENKENNFELD::from_f64(
        &m.name,
        values_2d,
        m.x_axis.clone(),
        m.y_axis.clone(),
        &x_axis_name,
        &y_axis_name,
        langname,
        &unit_w,
        &unit_x,
        &unit_y,
    )))
}
