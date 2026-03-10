use crate::DcmData;
use crate::value::Value;
use crate::block::Block;
use log::{warn, info};
use serde::Serialize;


#[derive(Debug, Serialize)]
pub enum DcmDiff {
    New{
        name: String,
    },
    Deleted{
        name: String,
    },
    Changed{
        name: String,
        old: Value,
        new: Value,
    },
    ChangedMap{
        name: String,
        old: String,
        new: String,
    }
}

pub fn dcm_diff(left: &DcmData, right: &DcmData) -> Vec<DcmDiff> {
    let mut diff = Vec::new();
    for (name, left_block) in left.blocks.iter() {
        if let Some(right_block) = right.blocks.get(name) {
            if left_block != right_block {
                diff.push(DcmDiff::Changed{
                    name: name.clone(),
                    old: left_block.get_values().clone(),
                    new: right_block.get_values().clone(),
                });
            }
        } else {
            diff.push(DcmDiff::Deleted{
                name: name.clone(),
            });
        }
    }
    for (name, right_block) in right.blocks.iter() {
        if !left.blocks.contains_key(name) {
            diff.push(DcmDiff::New{
                name: name.clone(),
            });
        } else {
            match (right_block, left.blocks.get(name).unwrap()) {
                (Block::Constant(v1), Block::Constant(v2)) => {
                    if v1 != v2 {
                        warn!("Block {} changed: from {} to {}", name, v1.value, v2.value);
                        diff.push(DcmDiff::Changed{
                            name: name.clone(),
                            old: v1.value.clone(),
                            new: v2.value.clone(),
                        });
                    } else { info!("Block unchanged:{}", name);}
                }
                (Block::ConstantBlock(v1), Block::ConstantBlock(v2)) => {
                    if v1 != v2 {
                        warn!("Block {} changed: from {} to {}", name, v1.value, v2.value);
                        diff.push(DcmDiff::Changed{
                            name: name.clone(),
                            old: v1.value.clone(),
                            new: v2.value.clone(),
                        });
                    }
                }
                (Block::Table(v1), Block::Table(v2)) => {
                    if v1 != v2 {
                        warn!("Block {} changed: from {} to {}", name, v1.value, v2.value);
                        diff.push(DcmDiff::Changed{
                            name: name.clone(),
                            old: v1.value.clone(),
                            new: v2.value.clone(),
                        });
                    }
                }
                (Block::Distribution(v1), Block::Distribution(v2)) => {
                    if v1 != v2 {
                        warn!("Block {} changed: from {} to {}", name, v1.value, v2.value);
                        diff.push(DcmDiff::Changed{
                            name: name.clone(),
                            old: v1.value.clone(),
                            new: v2.value.clone(),
                        });
                    }
                }
                (Block::Map(v1), Block::Map(v2)) => {
                    if v1 != v2 {
                        v1.show_diff(v2);
                        // warn!("Block {} changed: from {} to {}", name, v1.value, v2.value);
                        diff.push(DcmDiff::ChangedMap{   // map is different, show all diff in json file
                            name: name.clone(),
                            old: serde_json::to_string_pretty(v1).unwrap(),
                            new: serde_json::to_string_pretty(v2).unwrap(),
                        });
                    }
                }
                (left, _) => {
                    let name = left.get_name();
                    warn!("Block type mismatch:{}", name);
                }
            }
        }
    }
    diff
}