use crate::DcmData;
use crate::value::Value;
use crate::block::Block;


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
                        diff.push(DcmDiff::Changed{
                            name: name.clone(),
                            old: v1.value.clone(),
                            new: v2.value.clone(),
                        });
                    }
                }
                (Block::ConstantBlock(v1), Block::ConstantBlock(v2)) => {
                    if v1 != v2 {
                        diff.push(DcmDiff::Changed{
                            name: name.clone(),
                            old: v1.value.clone(),
                            new: v2.value.clone(),
                        });
                    }
                }
                (Block::Table(v1), Block::Table(v2)) => {
                    if v1 != v2 {
                        diff.push(DcmDiff::Changed{
                            name: name.clone(),
                            old: v1.value.clone(),
                            new: v2.value.clone(),
                        });
                    }
                }
                (Block::Distribution(v1), Block::Distribution(v2)) => {
                    if v1 != v2 {
                        diff.push(DcmDiff::Changed{
                            name: name.clone(),
                            old: v1.value.clone(),
                            new: v2.value.clone(),
                        });
                    }
                }
                (Block::Map(v1), Block::Map(v2)) => {
                    if v1 != v2 {
                        diff.push(DcmDiff::Changed{
                            name: name.clone(),
                            old: v1.value_flat.clone(),
                            new: v2.value_flat.clone(),
                        });
                    }
                }
                _ => {}
            }
        }
    }
    diff
}