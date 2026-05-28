pub mod attr;
pub mod block;
pub mod blocks;
pub mod diff;
pub mod gen;
pub mod value;

pub use diff::{
    dcm_diff, dcm_diff_with_metadata, validate_and_build_sources, CalSource, DcmDiff,
    DcmDiffResult, DiffMetadata, DiffSummary, MapAttr, MapChangeDetail, MapValues,
};

use block::Block;
use blocks::{
    FESTWERT, FESTWERTEBLOCK, GRUPPENKENNFELD, GRUPPENKENNLINIE, STUETZSTELLENVERTEILUNG,
};
use chrono::prelude::*;
use handlebars::*;
use indexmap::IndexMap;
use log::{info, warn};
use regex::Regex;
use serde_json::json;
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use value::Value;

/* handler bars helper */
fn dcm_vector_writer(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let vector_type = h.param(0).unwrap().value().render();
    let vector = h.param(1).unwrap().value();
    if let serde_json::Value::Array(arr) = vector {
        out.write("   ")?;
        let line_num = arr.len() / 6;
        let multi_line = arr.len() > 6;
        let text_flag = arr.iter().all(|x| x.is_string());
        let identifier = if text_flag && vector_type.as_str() == "WERT" {
            "TEXT" // convient solution for text values
        } else {
            vector_type.as_str()
        };
        for (index, val) in arr.chunks(6).enumerate() {
            out.write(format!("{}   ", identifier).as_str())?;
            for j in val {
                if text_flag {
                    out.write(format!("\"{}\"", j.as_str().unwrap_or("")).as_str())?;
                // add double quotes for text values
                } else {
                    let number = j.as_f64().unwrap_or(0.0);
                    let number_str = format!("{:.17}", number);
                    out.write(number_str.as_str())?;
                }
                out.write("   ")?;
            }
            if multi_line && line_num >= 1 && index < line_num {
                out.write("\r\n   ")?;
            }
        }
    }
    Ok(())
}

#[derive(PartialEq)]
pub enum AxisType {
    X,
    Y,
}

pub struct DcmData {
    pub blocks: IndexMap<String, Block>,
    pub source_path: Option<PathBuf>,
}

impl DcmData {
    pub fn from_blocks(blocks: IndexMap<String, Block>) -> Self {
        DcmData { blocks, source_path: None }
    }

    pub fn new(path: &Path) -> Self {
        // shall not return error here
        let mut file = File::open(path).unwrap();
        let reader = BufReader::new(&mut file);
        let all_lines: Vec<String> = reader.lines().map(|l| l.unwrap_or("".to_string())).collect(); // shall not return error if coding is not UTF-8
        let source_path = Some(path.to_path_buf());
        let mut blocks = IndexMap::new();

        let mut idx = 0;
        let file_path = path.to_string_lossy().to_string();
        while idx < all_lines.len() {
            let block_start = idx + 1; // 1-based line number
            let mut block = Vec::new();
            if all_lines[idx].starts_with("FESTWERT ") {
                // constant
                block.push(all_lines[idx].clone());
                idx += 1;
                while idx < all_lines.len() && !all_lines[idx].trim_start().starts_with("END") {
                    block.push(all_lines[idx].clone());
                    idx += 1;
                }
                if idx < all_lines.len() {
                    idx += 1; // skip END line
                }
                let text = block.join("\n");
                let constant = FESTWERT::parse_with_context(&text, Some(&file_path), block_start)
                    .expect("Failed to parse FESTWERT");
                blocks
                    .entry(constant.name.clone())
                    .and_modify(|_| warn!("[{}:{}] Duplicate constant name: {}", file_path, block_start, constant.name))
                    .or_insert(Block::Constant(constant));
            } else if all_lines[idx].starts_with("TEXTSTRING ") {
                // text string constant
                block.push(all_lines[idx].clone());
                idx += 1;
                while idx < all_lines.len() && !all_lines[idx].trim_start().starts_with("END") {
                    block.push(all_lines[idx].clone());
                    idx += 1;
                }
                if idx < all_lines.len() {
                    idx += 1; // skip END line
                }
                let text = block.join("\n");
                let constant = FESTWERT::parse_with_context(&text, Some(&file_path), block_start)
                    .expect("Failed to parse TEXTSTRING");
                blocks
                    .entry(constant.name.clone())
                    .and_modify(|_| warn!("[{}:{}] Duplicate constant name: {}", file_path, block_start, constant.name))
                    .or_insert(Block::Constant(constant));
            } else if all_lines[idx].starts_with("GRUPPENKENNLINIE ") {
                // one dimensional table
                block.push(all_lines[idx].clone());
                idx += 1;
                while idx < all_lines.len() && !all_lines[idx].trim_start().starts_with("END") {
                    block.push(all_lines[idx].clone());
                    idx += 1;
                }
                if idx < all_lines.len() {
                    idx += 1; // skip END line
                }
                let text = block.join("\n");
                let table = GRUPPENKENNLINIE::parse_with_context(&text, Some(&file_path), block_start)
                    .expect("Failed to parse GRUPPENKENNLINIE");
                blocks
                    .entry(table.name.clone())
                    .and_modify(|_| warn!("[{}:{}] Duplicate table name: {}", file_path, block_start, table.name))
                    .or_insert(Block::Table(table));
            } else if all_lines[idx].starts_with("STUETZSTELLENVERTEILUNG ") {
                // distribution axis
                block.push(all_lines[idx].clone());
                idx += 1;
                while idx < all_lines.len() && !all_lines[idx].trim_start().starts_with("END") {
                    block.push(all_lines[idx].clone());
                    idx += 1;
                }
                if idx < all_lines.len() {
                    idx += 1; // skip END line
                }
                let text = block.join("\n");
                let distribution =
                    STUETZSTELLENVERTEILUNG::parse_with_context(&text, Some(&file_path), block_start)
                        .expect("Failed to parse STUETZSTELLENVERTEILUNG");
                blocks
                    .entry(distribution.name.clone())
                    .and_modify(|_| {
                        warn!("[{}:{}] Duplicate distribution name: {}", file_path, block_start, distribution.name)
                    })
                    .or_insert(Block::Distribution(distribution));
            } else if all_lines[idx].starts_with("GRUPPENKENNFELD ") {
                block.push(all_lines[idx].clone());
                idx += 1;
                while idx < all_lines.len() && !all_lines[idx].trim_start().starts_with("END") {
                    block.push(all_lines[idx].clone());
                    idx += 1;
                }
                if idx < all_lines.len() {
                    idx += 1; // skip END line
                }
                let text = block.join("\n");
                let map = GRUPPENKENNFELD::parse_with_context(&text, Some(&file_path), block_start)
                    .expect("Failed to parse GRUPPENKENNFELD");
                blocks
                    .entry(map.name.clone())
                    .and_modify(|_| warn!("[{}:{}] Duplicate map name: {}", file_path, block_start, map.name))
                    .or_insert(Block::Map(map));
            } else if all_lines[idx].starts_with("FESTWERTEBLOCK ") {
                block.push(all_lines[idx].clone());
                idx += 1;
                while idx < all_lines.len() && !all_lines[idx].trim_start().starts_with("END") {
                    block.push(all_lines[idx].clone());
                    idx += 1;
                }
                if idx < all_lines.len() {
                    idx += 1; // skip END line
                }
                let text = block.join("\n");
                let fbl = FESTWERTEBLOCK::parse_with_context(&text, Some(&file_path), block_start)
                    .expect("Failed to parse FESTWERTEBLOCK");
                blocks
                    .entry(fbl.name.clone())
                    .and_modify(|_| warn!("[{}:{}] Duplicate map name: {}", file_path, block_start, fbl.name))
                    .or_insert(Block::ConstantBlock(fbl));
            } else {
                idx += 1; // skip unrecognized line
            }
        }
        DcmData { blocks, source_path }
    }

    pub fn get_all_variable_names(&self) -> Vec<String> {
        self.blocks.iter().map(|block| block.0.clone()).collect()
    }

    pub fn insert_block(&mut self, block: Block) {
        match &block {
            Block::ConstantBlock(cb) => {
                self.blocks.insert(cb.name.clone(), block);
            }
            Block::Distribution(dis) => {
                self.blocks.insert(dis.name.clone(), block);
            }
            Block::Constant(con) => {
                self.blocks.insert(con.name.clone(), block);
            }
            Block::Table(tab) => {
                self.blocks.insert(tab.name.clone(), block);
            }
            Block::Map(map) => {
                self.blocks.insert(map.name.clone(), block);
            }
        }
    }

    pub fn contains_block(&self, block_name: &str) -> bool {
        self.blocks.contains_key(block_name)
    }

    pub fn render_to_file(&self, file: &Path) {
        write_dcm_data(self, file);
    }

    pub fn filter_include(&mut self, include_pats: &[String]) {
        // filter blocks by regex pattern: only include blocks that match one of the patterns
        let patterns = include_pats
            .iter()
            .map(|p| Regex::new(p).unwrap())
            .collect::<Vec<Regex>>();
        let mut keys_to_keep = Vec::new();
        for (key, _) in self.blocks.iter() {
            if patterns.iter().any(|p| p.is_match(key)) {
                keys_to_keep.push(key.clone());
            }
        }
        self.blocks.retain(|k, _| keys_to_keep.contains(k));
    }

    pub fn filter_exclude(&mut self, exclude_pats: &[String]) {
        // filter blocks by regex pattern: exclude blocks that match one of the patterns
        let patterns = exclude_pats
            .iter()
            .map(|p| Regex::new(p).unwrap())
            .collect::<Vec<Regex>>();
        let mut keys_to_remove = Vec::new();
        for (key, _block) in self.blocks.iter_mut() {
            if patterns.iter().any(|p| p.is_match(key)) {
                keys_to_remove.push(key.clone());
            }
        }
        self.blocks.retain(|k, _| !keys_to_remove.contains(k));
    }
}

pub fn merge_dcm_data(main: &mut DcmData, others: Vec<DcmData>) {
    /*
    if the same block name exists in both dcms, the block in the others will be discarded.
    */
    for data in others.iter() {
        for (key, block) in &data.blocks {
            if !main.contains_block(key.as_str()) {
                main.blocks.insert(key.clone(), block.clone());
            } else {
                info!("Block : {} already exists in main dcm data, skipping", key);
            }
        }
    }
}

pub fn update_dcm_data(main: &mut DcmData, others: Vec<DcmData>) {
    /*
    if the same block name exists in both dcms, the block in the others will be used to update main dcm.
    */
    for data in others.iter() {
        for (key, block) in &data.blocks {
            if main.contains_block(key.as_str()) {
                main.blocks.insert(key.clone(), block.clone()); // this will overwrite the existing block
                info!("Block : {} updated in dcm data", key);
            } else {
                info!("Block : {} does not exist in main dcm data, skipping", key);
            }
        }
    }
}

fn sanitize_desc(raw: &str) -> String {
    raw.replace('\r', "").replace('\n', "")
}

pub fn write_dcm_data(data: &DcmData, file: &Path) {
    /* handlebars template  */
    let mut reg = Handlebars::new();
    let template = include_str!("../templates/dcm_template.hbs");
    reg.register_template_string("dcm_file", template).unwrap();
    reg.register_helper("dcm_vector", Box::new(dcm_vector_writer));

    /* data  */
    let mut constants = Vec::new();
    let mut textstrings = Vec::new();
    let mut constant_blocks = Vec::new();
    let mut table_blocks = Vec::new();
    let mut axis = Vec::new();
    let mut maps = Vec::new();
    for (key, blk) in &data.blocks {
        match blk {
            Block::Constant(c) => {
                if let Value::TEXT(_) = &c.value {
                    textstrings.push(json!({
                        "name": key,
                        "value": c.value,
                        "desc": sanitize_desc(&blk.get_desc().unwrap_or_default()),
                        "unit": blk.get_w_unit().unwrap_or("unitless".to_string()),
                    }))
                } else {
                    constants.push(json!({
                        "name": key,
                        "value": c.value,
                        "desc": sanitize_desc(&blk.get_desc().unwrap_or_default()),
                        "unit": blk.get_w_unit().unwrap_or("unitless".to_string()),
                    }))
                }
            }
            Block::ConstantBlock(c) => constant_blocks.push(json!( {
                "name": key,
                "value": c.value,
                "desc": sanitize_desc(&blk.get_desc().unwrap_or_default()),
                "unit": blk.get_w_unit().unwrap_or("unitless".to_string()),
                "dim": c.dim,
            })),
            Block::Table(t) => {
                let dim = t.value.len();
                table_blocks.push(json!( {
                    "name": key.clone(),
                    "value": t.value,
                    "desc": sanitize_desc(&blk.get_desc().unwrap_or_default()),
                    "unit_w": blk.get_w_unit().unwrap_or("na".to_string()),
                    "dim": dim,
                    "axis": t.axis.clone(),
                    "unit_x": blk.get_x_unit().unwrap_or("na".to_string()),
                    "axis_name": blk.get_x_var_name().unwrap_or("".to_string()),
                }))
            }
            Block::Distribution(d) => axis.push(json!({
                "name": key,
                "value": d.value.try_into_f64().unwrap_or(&vec![0f64]).clone(),
                "desc": sanitize_desc(&blk.get_desc().unwrap_or_default()),
                "unit": blk.get_x_unit().unwrap_or("na".to_string()),
                "dim": d.dim,
            })),
            Block::Map(m) => {
                let (dim_x, dim_y) = m.dim;
                let value_block: Vec<(&value::Value, value::Value)> = m
                    .value
                    .iter()
                    .zip(m.y_axis.iter())
                    .map(|(x, y)| (x, value::Value::WERT(vec![*y])))
                    .collect();
                maps.push(json!({
                    "name": key,
                    "value_block": value_block,
                    "desc": sanitize_desc(&blk.get_desc().unwrap_or_default()),
                    "unit_x": blk.get_x_unit().unwrap_or("na".to_string()),
                    "unit_y": blk.get_y_unit().unwrap_or("na".to_string()),
                    "unit_w": blk.get_w_unit().unwrap_or("na".to_string()),
                    "dim_x": dim_x,
                    "dim_y": dim_y,
                    "axis_x": m.x_axis,
                    "axis_x_name": blk.get_x_var_name().unwrap_or("".to_string()),
                    "axis_y_name": blk.get_y_var_name().unwrap_or("".to_string()),
                }))
            }
        }
    }
    let username = env::var("USERNAME").unwrap_or("unknown".to_string());
    let time = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let data_dict = json!({
        "creator": username,
        "timestamp": time,
        "constant": constants,
        "textstring": textstrings,
        "constantblock": constant_blocks,
        "tables": table_blocks,
        "axis": axis,
        "map": maps,
    });

    #[cfg(debug_assertions)]
    {
        println!("In debug build. will generate json file for debug purpose.");
        let file = File::create("./output/data.json").unwrap();
        serde_json::to_writer_pretty(file, &data_dict).unwrap();
    }
    let writer = File::create(file).unwrap();
    reg.render_to_write("dcm_file", &data_dict, writer).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    #[rstest]
    fn test_dcm_write() {
        let file_to_read = "./test-dcms/test_sample_673.DCM";
        let file_to_write = "./output/test.DCM";
        let dcm_data = DcmData::new(Path::new(file_to_read));
        let path = Path::new(file_to_write);
        write_dcm_data(&dcm_data, path);
    }

    #[rstest]
    fn test_dcm_write1() {
        let file_to_read = "./test-dcms/test1.DCM";
        let file_to_write = "./output/test1_gen.DCM";
        let dcm_data = DcmData::new(Path::new(file_to_read));
        let path = Path::new(file_to_write);
        write_dcm_data(&dcm_data, path);
    }

    #[rstest]
    fn test_festwert() {
        let value = 1.0;
        let name = "CALIBRATION_PARAMETER".to_string();
        let desc = "calibration parameter".to_string();
        let unit = "mm".to_string();
        let festwert = FESTWERT::from_f64(name.clone(), value, desc, unit);
        let mut blocks = IndexMap::new();
        blocks.insert(name, Block::Constant(festwert));
        let dcm_data = DcmData { blocks, source_path: None };
        let path = Path::new("./output/festwert.DCM");
        write_dcm_data(&dcm_data, path);
    }
}
