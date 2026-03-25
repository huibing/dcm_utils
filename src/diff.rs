use crate::DcmData;
use crate::value::Value;
use crate::block::Block;
use log::info;
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

    // Find deleted blocks (in left but not in right)
    for (name, _) in left.blocks.iter() {
        if !right.blocks.contains_key(name) {
            diff.push(DcmDiff::Deleted{
                name: name.clone(),
            });
        }
    }

    // Find new and changed blocks (in right)
    for (name, right_block) in right.blocks.iter() {
        match left.blocks.get(name) {
            None => {
                // Block exists in right but not in left - it's new
                diff.push(DcmDiff::New{
                    name: name.clone(),
                });
            }
            Some(left_block) => {
                // Block exists in both - check if changed
                if left_block != right_block {
                    info!("Block {} changed", name);
                    match (left_block, right_block) {
                        (Block::Map(left_map), Block::Map(right_map)) => {
                            diff.push(DcmDiff::ChangedMap{
                                name: name.clone(),
                                old: serde_json::to_string_pretty(left_map).unwrap(),
                                new: serde_json::to_string_pretty(right_map).unwrap(),
                            });
                        }
                        _ => {
                            diff.push(DcmDiff::Changed{
                                name: name.clone(),
                                old: left_block.get_values().clone(),
                                new: right_block.get_values().clone(),
                            });
                        }
                    }
                }
            }
        }
    }

    diff
}