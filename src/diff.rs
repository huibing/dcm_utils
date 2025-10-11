use crate::DcmData;
use crate::value::Value;


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
        }
    }
    diff
}