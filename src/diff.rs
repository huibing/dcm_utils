use crate::DcmData;
use crate::value::Value;
use crate::block::Block;
use crate::blocks::GRUPPENKENNFELD;
use crate::attr::string_attr::StringAttr;
use log::info;
use serde::{Serialize, Deserialize};
use std::path::Path;
use std::time::SystemTime;

/// Metadata about the diff operation
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DiffMetadata {
    pub original_file: String,
    pub modified_file: String,
    pub timestamp: String,
}

impl DiffMetadata {
    pub fn new(original: &Path, modified: &Path) -> Self {
        Self {
            original_file: original.display().to_string(),
            modified_file: modified.display().to_string(),
            timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_secs().to_string())
                .unwrap_or_else(|_| "0".to_string()),
        }
    }
}

/// Summary of differences
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DiffSummary {
    pub new_count: usize,
    pub deleted_count: usize,
    pub changed_count: usize,
    pub total: usize,
}

impl DiffSummary {
    pub fn from_differences(differences: &[DcmDiff]) -> Self {
        let new_count = differences.iter().filter(|d| matches!(d, DcmDiff::New { .. })).count();
        let deleted_count = differences.iter().filter(|d| matches!(d, DcmDiff::Deleted { .. })).count();
        let changed_count = differences.iter().filter(|d| matches!(d, DcmDiff::Changed { .. } | DcmDiff::ChangedMap { .. })).count();

        Self {
            new_count,
            deleted_count,
            changed_count,
            total: differences.len(),
        }
    }
}

/// Complete diff result with metadata and differences
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DcmDiffResult {
    pub metadata: DiffMetadata,
    pub summary: DiffSummary,
    pub differences: Vec<DcmDiff>,
}

/// Represents a map attribute (identifier + value)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MapAttr {
    pub identifier: String,
    pub value: String,
}

impl From<&StringAttr> for MapAttr {
    fn from(attr: &StringAttr) -> Self {
        Self {
            identifier: attr.identifier.clone(),
            value: attr.value.clone(),
        }
    }
}

/// Represents the values of a 2D map in a structured way
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MapValues {
    pub name: String,
    pub attrs: Vec<MapAttr>,
    /// X dimension (number of columns)
    pub dim_x: usize,
    /// Y dimension (number of rows)
    pub dim_y: usize,
    /// All values as a flat array (row-major order)
    pub values: Vec<f64>,
    /// Optional 2D array representation of values
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values_2d: Option<Vec<Vec<f64>>>,
    /// X-axis variable name
    pub x_axis_name: String,
    /// Y-axis variable name
    pub y_axis_name: String,
    /// X-axis breakpoint values
    pub x_axis: Vec<f64>,
    /// Y-axis breakpoint values
    pub y_axis: Vec<f64>,
}

impl From<&GRUPPENKENNFELD> for MapValues {
    fn from(map: &GRUPPENKENNFELD) -> Self {
        // Extract flat values from value_flat (which is a Value enum)
        let values: Vec<f64> = match &map.value_flat {
            Value::WERT(v) => v.clone(),
            _ => Vec::new(),
        };

        // Convert 2D values (Vec<Value>) to Vec<Vec<f64>>
        let values_2d: Vec<Vec<f64>> = map.value.iter()
            .filter_map(|v| match v {
                Value::WERT(row) => Some(row.clone()),
                _ => None,
            })
            .collect();

        Self {
            name: map.name.clone(),
            attrs: map.attrs.iter().map(|a| a.into()).collect(),
            dim_x: map.dim.0,
            dim_y: map.dim.1,
            values,
            values_2d: Some(values_2d),
            x_axis_name: map.x_axis_name.clone(),
            y_axis_name: map.y_axis_name.clone(),
            x_axis: map.x_axis.clone(),
            y_axis: map.y_axis.clone(),
        }
    }
}

/// Detailed change information for a 2D map
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MapChangeDetail {
    pub old_values: MapValues,
    pub new_values: MapValues,
}

impl DcmDiffResult {
    pub fn new(metadata: DiffMetadata, differences: Vec<DcmDiff>) -> Self {
        let summary = DiffSummary::from_differences(&differences);
        Self {
            metadata,
            summary,
            differences,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum DcmDiff {
    New{
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
    },
    Deleted{
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
    },
    Changed{
        name: String,
        old: Value,
        new: Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
    },
    ChangedMap{
        name: String,
        detail: MapChangeDetail,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
    }
}

pub fn dcm_diff(left: &DcmData, right: &DcmData) -> Vec<DcmDiff> {
    dcm_diff_with_details(left, right, false)
}

/// Internal function to compute diff with optional detailed descriptions
fn dcm_diff_with_details(left: &DcmData, right: &DcmData, _detailed: bool) -> Vec<DcmDiff> {
    let mut diff = Vec::new();

    // Find deleted blocks (in left but not in right)
    for (name, left_block) in left.blocks.iter() {
        if !right.blocks.contains_key(name) {
            let description = format!("Deleted {} block '{}'", block_type_name(left_block), name);
            diff.push(DcmDiff::Deleted{
                name: name.clone(),
                description: Some(description),
            });
        }
    }

    // Find new and changed blocks (in right)
    for (name, right_block) in right.blocks.iter() {
        match left.blocks.get(name) {
            None => {
                // Block exists in right but not in left - it's new
                let description = format!("New {} block '{}'", block_type_name(right_block), name);
                diff.push(DcmDiff::New{
                    name: name.clone(),
                    description: Some(description),
                });
            }
            Some(left_block) => {
                // Block exists in both - check if changed
                if left_block != right_block {
                    info!("Block {} changed", name);
                    let description = generate_change_description(name, left_block, right_block);
                    match (left_block, right_block) {
                        (Block::Map(left_map), Block::Map(right_map)) => {
                            let detail = MapChangeDetail {
                                old_values: MapValues::from(left_map),
                                new_values: MapValues::from(right_map),
                            };
                            diff.push(DcmDiff::ChangedMap{
                                name: name.clone(),
                                detail,
                                description: Some(description),
                            });
                        }
                        _ => {
                            diff.push(DcmDiff::Changed{
                                name: name.clone(),
                                old: left_block.get_values().clone(),
                                new: right_block.get_values().clone(),
                                description: Some(description),
                            });
                        }
                    }
                }
            }
        }
    }

    diff
}

/// Compute diff with metadata including file paths
pub fn dcm_diff_with_metadata(left: &DcmData, right: &DcmData, original_path: &Path, modified_path: &Path) -> DcmDiffResult {
    let metadata = DiffMetadata::new(original_path, modified_path);
    let differences = dcm_diff_with_details(left, right, true);
    DcmDiffResult::new(metadata, differences)
}

/// Helper function to get block type name
fn block_type_name(block: &Block) -> &'static str {
    match block {
        Block::Constant(_) => "FESTWERT",
        Block::ConstantBlock(_) => "FESTWERTEBLOCK",
        Block::Table(_) => "GRUPPENKENNLINIE",
        Block::Distribution(_) => "STUETZSTELLENVERTEILUNG",
        Block::Map(_) => "GRUPPENKENNFELD",
    }
}

/// Generate a description of what changed between two blocks
fn generate_change_description(name: &str, left: &Block, right: &Block) -> String {
    match (left, right) {
        (Block::Table(left_table), Block::Table(right_table)) => {
            let mut changes = Vec::new();
            if left_table.axis.len() != right_table.axis.len() {
                changes.push(format!("axis points: {} -> {}", left_table.axis.len(), right_table.axis.len()));
            }
            if left_table.axis_var_name != right_table.axis_var_name {
                changes.push(format!("axis var: {} -> {}", left_table.axis_var_name, right_table.axis_var_name));
            }
            if left_table.value != right_table.value {
                changes.push("values changed".to_string());
            }
            if changes.is_empty() {
                format!("GRUPPENKENNLINIE '{}' changed", name)
            } else {
                format!("GRUPPENKENNLINIE '{}' changed: {}", name, changes.join(", "))
            }
        }
        (Block::Map(left_map), Block::Map(right_map)) => {
            let mut changes = Vec::new();
            if left_map.dim != right_map.dim {
                changes.push(format!("dimensions: {:?} -> {:?}", left_map.dim, right_map.dim));
            }
            if left_map.x_axis_name != right_map.x_axis_name {
                changes.push(format!("X-axis var: {} -> {}", left_map.x_axis_name, right_map.x_axis_name));
            }
            if left_map.y_axis_name != right_map.y_axis_name {
                changes.push(format!("Y-axis var: {} -> {}", left_map.y_axis_name, right_map.y_axis_name));
            }
            if left_map.value_flat != right_map.value_flat {
                changes.push("values changed".to_string());
            }
            if changes.is_empty() {
                format!("GRUPPENKENNFELD '{}' changed", name)
            } else {
                format!("GRUPPENKENNFELD '{}' changed: {}", name, changes.join(", "))
            }
        }
        (Block::ConstantBlock(left_cb), Block::ConstantBlock(right_cb)) => {
            if left_cb.value != right_cb.value {
                format!("FESTWERTEBLOCK '{}' values changed", name)
            } else {
                format!("FESTWERTEBLOCK '{}' changed", name)
            }
        }
        (Block::Constant(left_c), Block::Constant(right_c)) => {
            if left_c.value != right_c.value {
                format!("FESTWERT '{}' value changed", name)
            } else {
                format!("FESTWERT '{}' changed", name)
            }
        }
        (Block::Distribution(left_d), Block::Distribution(right_d)) => {
            if left_d.value != right_d.value {
                format!("STUETZSTELLENVERTEILUNG '{}' points changed", name)
            } else {
                format!("STUETZSTELLENVERTEILUNG '{}' changed", name)
            }
        }
        _ => {
            format!("Block '{}' type changed", name)
        }
    }
}