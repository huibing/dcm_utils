pub mod attr;
pub mod block;

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use block::{FESTWERT, GRUPPENKENNLINIE, STUETZSTELLENVERTEILUNG, GRUPPENKENNFELD, FESTWERTEBLOCK, Block};
use indexmap::IndexMap;
use log::warn;

#[derive(PartialEq)]
pub enum AxisType {
    X,
    Y
}

pub struct DcmData {
    pub blocks: IndexMap<String, Block>
}

impl DcmData {
    pub fn new(path: &Path) -> Self {  // shall not return error here
        let mut file = File::open(path).unwrap();
        let reader = BufReader::new(&mut file);
        let mut blocks = IndexMap::new();

        let mut lines = reader.lines().map(|l| l.unwrap_or("".to_string())); // shall not return error if coding is not UTF-8
        let mut line = lines.next();
        while line.is_some() {
            let mut block = Vec::new();
            if line.as_ref().unwrap().starts_with("FESTWERT ") {    // constant
                loop {
                    block.push(line.unwrap());
                    line = lines.next();
                    if line.is_none() || line.as_ref().unwrap().starts_with("END") {
                        let constant = block.join("\n").parse::<FESTWERT>().unwrap();
                        blocks.entry(constant.name.clone())
                        .and_modify( |_| warn!("Duplicate constant name: {}", constant.name))
                        .or_insert(Block::Constant(constant));
                        break;
                    }
                }
            } else if line.as_ref().unwrap().starts_with("GRUPPENKENNLINIE ") {   // one dimensional table
                loop {
                    block.push(line.unwrap());
                    line = lines.next();
                    if line.is_none() || line.as_ref().unwrap().starts_with("END") {
                        let table = block.join("\n").parse::<GRUPPENKENNLINIE>().unwrap();
                        blocks.entry(table.name.clone())
                        .and_modify( |_| warn!("Duplicate table name: {}", table.name))
                        .or_insert(Block::Table(table));
                        break;
                    }
                }
            } else if line.as_ref().unwrap().starts_with("STUETZSTELLENVERTEILUNG ") {   // distribution axis
                loop {
                    block.push(line.unwrap());
                    line = lines.next();
                    if line.is_none() || line.as_ref().unwrap().starts_with("END") {
                        let distribution = block.join("\n").parse::<STUETZSTELLENVERTEILUNG>().unwrap();
                        blocks.entry(distribution.name.clone())
                            .and_modify( |_| warn!("Duplicate distribution name: {}", distribution.name))
                            .or_insert(Block::Distribution(distribution));
                        break;
                    }
                }
            } else if line.as_ref().unwrap().starts_with("GRUPPENKENNFELD ") {
                loop {
                    block.push(line.unwrap());
                    line = lines.next();
                    if line.is_none() || line.as_ref().unwrap().starts_with("END") {
                        let map = block.join("\n").parse::<GRUPPENKENNFELD>().unwrap();
                        blocks.entry(map.name.clone())
                            .and_modify( |_| warn!("Duplicate map name: {}", map.name))
                            .or_insert(Block::Map(map));
                        break;
                    }
                }
            } else if line.as_ref().unwrap().starts_with("FESTWERTEBLOCK "){
                loop {
                    block.push(line.unwrap());
                    line = lines.next();
                    if line.is_none() || line.as_ref().unwrap().starts_with("END") {
                        let fbl = block.join("\n").parse::<FESTWERTEBLOCK>().unwrap();
                        blocks.entry(fbl.name.clone())
                            .and_modify( |_| warn!("Duplicate map name: {}", fbl.name))
                            .or_insert(Block::ConstantBlock(fbl));
                        break;
                    }
                }
            } else {
                line = lines.next();
            }
        }
        DcmData {
            blocks
        }
    }

    pub fn get_all_variable_names(&self) -> Vec<String> {
        self.blocks.iter().map(|block| block.0.clone()).collect()
    }

}
