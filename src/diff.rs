use crate::attr::string_attr::StringAttr;
use crate::block::Block;
use crate::blocks::GRUPPENKENNFELD;
use crate::gen::gen_dcm_data;
use crate::value::Value;
use crate::DcmData;
use log::info;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub enum CalSource {
    Dcm(PathBuf),
    A2lHex { a2l: PathBuf, hex: PathBuf },
}

impl CalSource {
    pub fn load(&self) -> Result<DcmData, Box<dyn Error>> {
        match self {
            CalSource::Dcm(path) => {
                let path = path.clone();
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| DcmData::new(&path)))
                    .map_err(|_| format!("Failed to load DCM file: {}", path.display()).into())
            }
            CalSource::A2lHex { a2l, hex } => gen_dcm_data(a2l, hex),
        }
    }

    pub fn label(&self) -> String {
        match self {
            CalSource::Dcm(path) => path.display().to_string(),
            CalSource::A2lHex { a2l, hex } => format!("{} + {}", a2l.display(), hex.display()),
        }
    }
}

/// Validate CLI flags and build ordered CalSource pair.
/// Returns Err(String) with a user-facing message on validation failure.
pub fn validate_and_build_sources(
    dcm: &[PathBuf],
    a2l: &[PathBuf],
    hex: &[PathBuf],
) -> Result<(CalSource, CalSource), String> {
    let total = dcm.len() + a2l.len();
    if total != 2 {
        return Err(format!(
            "Expected exactly 2 sources (--dcm and/or --a2l+--hex), got {}",
            total
        ));
    }
    if a2l.len() != hex.len() {
        return Err(format!(
            "Mismatched --a2l ({} provided) and --hex ({} provided) flags",
            a2l.len(),
            hex.len()
        ));
    }

    let mut sources = Vec::new();
    // A2L+HEX first (reference/original from ECU flash), then DCM (candidate)
    for (a, h) in a2l.iter().zip(hex.iter()) {
        sources.push(CalSource::A2lHex {
            a2l: a.clone(),
            hex: h.clone(),
        });
    }
    for path in dcm {
        sources.push(CalSource::Dcm(path.clone()));
    }
    let right = sources.pop().unwrap();
    let left = sources.pop().unwrap();
    Ok((left, right))
}

/// Metadata about the diff operation
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DiffMetadata {
    #[serde(rename = "original_file")]
    pub left_label: String,
    #[serde(rename = "modified_file")]
    pub right_label: String,
    pub timestamp: String,
}

impl DiffMetadata {
    pub fn new(left_label: &str, right_label: &str) -> Self {
        Self {
            left_label: left_label.to_string(),
            right_label: right_label.to_string(),
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
        let new_count = differences
            .iter()
            .filter(|d| matches!(d, DcmDiff::New { .. }))
            .count();
        let deleted_count = differences
            .iter()
            .filter(|d| matches!(d, DcmDiff::Deleted { .. }))
            .count();
        let changed_count = differences
            .iter()
            .filter(|d| matches!(d, DcmDiff::Changed { .. } | DcmDiff::ChangedMap { .. }))
            .count();

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
        let values_2d: Vec<Vec<f64>> = map
            .value
            .iter()
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
    New {
        name: String,
        value: Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        axis: Option<Vec<f64>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        axis_var_name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
    },
    Deleted {
        name: String,
        value: Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        axis: Option<Vec<f64>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        axis_var_name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
    },
    Changed {
        name: String,
        old: Value,
        new: Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        old_axis: Option<Vec<f64>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        new_axis: Option<Vec<f64>>,
    },
    ChangedMap {
        name: String,
        detail: MapChangeDetail,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
    },
}

pub fn dcm_diff(left: &DcmData, right: &DcmData) -> Vec<DcmDiff> {
    dcm_diff_with_details(left, right, false)
}

/// Internal function to compute diff with optional detailed descriptions
fn dcm_diff_with_details(
    left: &DcmData,
    right: &DcmData,
    _detailed: bool,
) -> Vec<DcmDiff> {
    let mut diff = Vec::new();

    // Find deleted blocks (in left but not in right)
    for (name, left_block) in left.blocks.iter() {
        if !right.blocks.contains_key(name) {
            let description = format!("Deleted {} block '{}'", block_type_name(left_block), name);
            let value = left_block.get_values().clone();
            let (axis, axis_var_name) = match left_block {
                Block::Table(t) => (Some(t.axis.clone()), Some(t.axis_var_name.clone())),
                _ => (None, None),
            };
            diff.push(DcmDiff::Deleted {
                name: name.clone(),
                value,
                axis,
                axis_var_name,
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
                let value = right_block.get_values().clone();
                let (axis, axis_var_name) = match right_block {
                    Block::Table(t) => (Some(t.axis.clone()), Some(t.axis_var_name.clone())),
                    _ => (None, None),
                };
                diff.push(DcmDiff::New {
                    name: name.clone(),
                    value,
                    axis,
                    axis_var_name,
                    description: Some(description),
                });
            }
            Some(left_block) => {
                // Block exists in both - check if changed using f32 byte comparison
                if !left_block.f32_bytes_eq(right_block) {
                    info!("Block {} changed", name);
                    let description =
                        generate_change_description(name, left_block, right_block);
                    match (left_block, right_block) {
                        (Block::Map(left_map), Block::Map(right_map)) => {
                            let detail = MapChangeDetail {
                                old_values: MapValues::from(left_map),
                                new_values: MapValues::from(right_map),
                            };
                            diff.push(DcmDiff::ChangedMap {
                                name: name.clone(),
                                detail,
                                description: Some(description),
                            });
                        }
                        (Block::Table(left_tbl), Block::Table(right_tbl)) => {
                            diff.push(DcmDiff::Changed {
                                name: name.clone(),
                                old: left_block.get_values().clone(),
                                new: right_block.get_values().clone(),
                                description: Some(description),
                                old_axis: Some(left_tbl.axis.clone()),
                                new_axis: Some(right_tbl.axis.clone()),
                            });
                        }
                        _ => {
                            diff.push(DcmDiff::Changed {
                                name: name.clone(),
                                old: left_block.get_values().clone(),
                                new: right_block.get_values().clone(),
                                description: Some(description),
                                old_axis: None,
                                new_axis: None,
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
pub fn dcm_diff_with_metadata(
    left: &DcmData,
    right: &DcmData,
    left_src: &CalSource,
    right_src: &CalSource,
) -> DcmDiffResult {
    let metadata = DiffMetadata::new(&left_src.label(), &right_src.label());
    let differences = dcm_diff_with_details(left, right, true);
    DcmDiffResult::new(metadata, differences)
}

// ===========================================================================
// Multi-Source Diff (diff-base)
// ===========================================================================

/// Metadata for a multi-source diff (diff-base) result
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MultiSourceDiffMetadata {
    /// Labels for all sources, index 0 is the base source
    pub sources: Vec<String>,
}

impl MultiSourceDiffMetadata {
    pub fn new(sources: &[String]) -> Self {
        Self {
            sources: sources.to_vec(),
        }
    }
}

/// Per-source value for a variable in multi-source diff
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MultiSourceVariableValue {
    /// The value (None if source doesn't have this variable)
    pub value: Option<Value>,
    /// Whether this variable exists in this source
    pub present: bool,
    /// X-axis breakpoints (for GRUPPENKENNFELD)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x_axis: Option<Vec<f64>>,
    /// Y-axis breakpoints (for GRUPPENKENNFELD)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y_axis: Option<Vec<f64>>,
}

/// A single variable difference across all sources in multi-source diff
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MultiSourceVariableDiff {
    /// Variable name
    pub name: String,
    /// Block type description
    pub block_type: String,
    /// Values from each source (index 0 = base)
    pub source_values: Vec<MultiSourceVariableValue>,
    /// X dimension (columns) for GRUPPENKENNFELD
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dim_x: Option<usize>,
    /// Y dimension (rows) for GRUPPENKENNFELD
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dim_y: Option<usize>,
    /// X-axis variable name (for GRUPPENKENNFELD)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x_axis_name: Option<String>,
    /// Y-axis variable name (for GRUPPENKENNFELD)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y_axis_name: Option<String>,
}

/// Result of a multi-source diff (diff-base) comparison
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MultiSourceDiffResult {
    pub metadata: MultiSourceDiffMetadata,
    /// Number of variables in the base source
    pub total_variables: usize,
    /// Number of variables with differences across sources
    pub variables_with_diffs: usize,
    /// Detailed differences
    pub differences: Vec<MultiSourceVariableDiff>,
}

/// Compute multi-source diff: compare all sources against a base source.
/// Only variables present in the base source are compared; variables not
/// in the base are ignored.
pub fn compute_multi_source_diff(sources: &[CalSource]) -> Result<MultiSourceDiffResult, Box<dyn Error>> {
    if sources.len() < 2 {
        return Err("At least 2 sources (base + at least 1 other) are required".into());
    }

    // Load all source data
    let all_data: Vec<DcmData> = sources
        .iter()
        .map(|s| s.load())
        .collect::<Result<Vec<_>, _>>()?;

    let labels: Vec<String> = sources.iter().map(|s| s.label()).collect();
    let base_data = &all_data[0];

    let mut differences = Vec::new();

    // For each variable in the base source
    for (var_name, base_block) in &base_data.blocks {
        let block_type = block_type_name(base_block);
        let base_value = base_block.get_values().clone();

        // Extract MAP metadata if applicable
        let (base_x_axis, base_y_axis, dim_x, dim_y, x_axis_name, y_axis_name) = match base_block {
            Block::Map(m) => (
                Some(m.x_axis.clone()),
                Some(m.y_axis.clone()),
                Some(m.dim.0),
                Some(m.dim.1),
                Some(m.x_axis_name.clone()),
                Some(m.y_axis_name.clone()),
            ),
            _ => (None, None, None, None, None, None),
        };

        let mut source_values: Vec<MultiSourceVariableValue> = Vec::with_capacity(sources.len());
        source_values.push(MultiSourceVariableValue {
            value: Some(base_value),
            present: true,
            x_axis: base_x_axis,
            y_axis: base_y_axis,
        });

        // Check each other source
        let mut has_diff = false;
        for other_data in &all_data[1..] {
            match other_data.blocks.get(var_name) {
                Some(other_block) => {
                    let other_value = other_block.get_values().clone();
                    // Compare with base using f32 bytes
                    if !base_block.f32_bytes_eq(other_block) {
                        has_diff = true;
                    }
                    let (ox, oy) = match other_block {
                        Block::Map(m) => (Some(m.x_axis.clone()), Some(m.y_axis.clone())),
                        _ => (None, None),
                    };
                    source_values.push(MultiSourceVariableValue {
                        value: Some(other_value),
                        present: true,
                        x_axis: ox,
                        y_axis: oy,
                    });
                }
                None => {
                    source_values.push(MultiSourceVariableValue {
                        value: None,
                        present: false,
                        x_axis: None,
                        y_axis: None,
                    });
                    has_diff = true; // missing variable counts as a difference
                }
            }
        }

        if has_diff {
            differences.push(MultiSourceVariableDiff {
                name: var_name.clone(),
                block_type: block_type.to_string(),
                source_values,
                dim_x,
                dim_y,
                x_axis_name,
                y_axis_name,
            });
        }
    }

    let total_variables = base_data.blocks.len();
    let variables_with_diffs = differences.len();

    Ok(MultiSourceDiffResult {
        metadata: MultiSourceDiffMetadata::new(&labels),
        total_variables,
        variables_with_diffs,
        differences,
    })
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
                changes.push(format!(
                    "axis points: {} -> {}",
                    left_table.axis.len(),
                    right_table.axis.len()
                ));
            }
            if left_table.axis_var_name != right_table.axis_var_name {
                changes.push(format!(
                    "axis var: {} -> {}",
                    left_table.axis_var_name, right_table.axis_var_name
                ));
            }
            if !left_table.value.f32_bytes_eq(&right_table.value) {
                changes.push("values changed".to_string());
            }
            if changes.is_empty() {
                format!("GRUPPENKENNLINIE '{}' changed", name)
            } else {
                format!(
                    "GRUPPENKENNLINIE '{}' changed: {}",
                    name,
                    changes.join(", ")
                )
            }
        }
        (Block::Map(left_map), Block::Map(right_map)) => {
            let mut changes = Vec::new();
            if left_map.dim != right_map.dim {
                changes.push(format!(
                    "dimensions: {:?} -> {:?}",
                    left_map.dim, right_map.dim
                ));
            }
            if left_map.x_axis_name != right_map.x_axis_name {
                changes.push(format!(
                    "X-axis var: {} -> {}",
                    left_map.x_axis_name, right_map.x_axis_name
                ));
            }
            if left_map.y_axis_name != right_map.y_axis_name {
                changes.push(format!(
                    "Y-axis var: {} -> {}",
                    left_map.y_axis_name, right_map.y_axis_name
                ));
            }
            if !left_map.value_flat.f32_bytes_eq(&right_map.value_flat) {
                changes.push("values changed".to_string());
            }
            if changes.is_empty() {
                format!("GRUPPENKENNFELD '{}' changed", name)
            } else {
                format!("GRUPPENKENNFELD '{}' changed: {}", name, changes.join(", "))
            }
        }
        (Block::ConstantBlock(left_cb), Block::ConstantBlock(right_cb)) => {
            if !left_cb.value.f32_bytes_eq(&right_cb.value) {
                format!("FESTWERTEBLOCK '{}' values changed", name)
            } else {
                format!("FESTWERTEBLOCK '{}' changed", name)
            }
        }
        (Block::Constant(left_c), Block::Constant(right_c)) => {
            if !left_c.value.f32_bytes_eq(&right_c.value) {
                format!("FESTWERT '{}' value changed", name)
            } else {
                format!("FESTWERT '{}' changed", name)
            }
        }
        (Block::Distribution(left_d), Block::Distribution(right_d)) => {
            if !left_d.value.f32_bytes_eq(&right_d.value) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_cal_source_label_dcm() {
        let src = CalSource::Dcm(PathBuf::from("test-dcms/example.DCM"));
        let label = src.label();
        assert!(
            label.contains("example.DCM"),
            "Label should contain the file name, got: {}",
            label
        );
    }

    #[test]
    fn test_cal_source_label_a2l_hex() {
        let src = CalSource::A2lHex {
            a2l: PathBuf::from("test-dcms/cal.a2l"),
            hex: PathBuf::from("test-dcms/flash.hex"),
        };
        let label = src.label();
        assert!(label.contains("cal.a2l"), "Label should contain a2l path");
        assert!(label.contains("flash.hex"), "Label should contain hex path");
        assert!(
            label.contains(" + "),
            "Label should contain ' + ' separator"
        );
    }

    #[test]
    fn test_validate_and_build_sources_two_dcm() {
        let dcm = vec![PathBuf::from("a.DCM"), PathBuf::from("b.DCM")];
        let (left, right) = validate_and_build_sources(&dcm, &[], &[]).unwrap();
        assert!(matches!(left, CalSource::Dcm(_)));
        assert!(matches!(right, CalSource::Dcm(_)));
    }

    #[test]
    fn test_validate_and_build_sources_mixed() {
        let dcm = vec![PathBuf::from("ref.DCM")];
        let a2l = vec![PathBuf::from("cal.a2l")];
        let hex = vec![PathBuf::from("flash.hex")];
        let (left, right) = validate_and_build_sources(&dcm, &a2l, &hex).unwrap();
        // A2L+HEX (ECU reference) comes first as left/old, DCM as right/new
        assert!(matches!(left, CalSource::A2lHex { .. }));
        assert!(matches!(right, CalSource::Dcm(_)));
    }

    #[test]
    fn test_validate_and_build_sources_two_a2l_hex() {
        let a2l = vec![PathBuf::from("v1.a2l"), PathBuf::from("v2.a2l")];
        let hex = vec![PathBuf::from("v1.hex"), PathBuf::from("v2.hex")];
        let (left, right) = validate_and_build_sources(&[], &a2l, &hex).unwrap();
        assert!(matches!(left, CalSource::A2lHex { .. }));
        assert!(matches!(right, CalSource::A2lHex { .. }));
    }

    #[test]
    fn test_validate_rejects_zero_sources() {
        let err = validate_and_build_sources(&[], &[], &[]).unwrap_err();
        assert!(err.contains("Expected exactly 2 sources"), "Got: {}", err);
    }

    #[test]
    fn test_validate_rejects_one_source() {
        let dcm = vec![PathBuf::from("a.DCM")];
        let err = validate_and_build_sources(&dcm, &[], &[]).unwrap_err();
        assert!(err.contains("Expected exactly 2 sources"));
    }

    #[test]
    fn test_validate_rejects_three_sources() {
        let dcm = vec![
            PathBuf::from("a.DCM"),
            PathBuf::from("b.DCM"),
            PathBuf::from("c.DCM"),
        ];
        let err = validate_and_build_sources(&dcm, &[], &[]).unwrap_err();
        assert!(err.contains("Expected exactly 2 sources"));
    }

    #[test]
    fn test_validate_rejects_four_sources() {
        let dcm = vec![
            PathBuf::from("a.DCM"),
            PathBuf::from("b.DCM"),
            PathBuf::from("c.DCM"),
            PathBuf::from("d.DCM"),
        ];
        let err = validate_and_build_sources(&dcm, &[], &[]).unwrap_err();
        assert!(err.contains("Expected exactly 2 sources"));
    }

    #[test]
    fn test_validate_rejects_mismatched_a2l_hex() {
        let a2l = vec![PathBuf::from("cal.a2l"), PathBuf::from("v2.a2l")];
        let hex = vec![PathBuf::from("flash.hex")]; // only 1 hex for 2 a2ls
        let err = validate_and_build_sources(&[], &a2l, &hex).unwrap_err();
        assert!(err.contains("Mismatched"));
    }

    use crate::block::Block;
    use crate::blocks::FESTWERT;
    use crate::DcmData;
    use indexmap::IndexMap;

    fn make_dcm_with_constant(name: &str, value: f64) -> DcmData {
        let festwert = FESTWERT::from_f64(
            name.to_string(),
            value,
            "desc".to_string(),
            "unit".to_string(),
        );
        let mut blocks = IndexMap::new();
        blocks.insert(name.to_string(), Block::Constant(festwert));
        DcmData { blocks, source_path: None }
    }

    #[test]
    fn test_dcm_diff_f32_bytes_suppresses_float_noise() {
        let left = make_dcm_with_constant("param1", 1.0);
        let right = make_dcm_with_constant("param1", 1.0 + 1e-9);

        // f32 byte comparison: should suppress the noise diff (1e-9 is below f32 precision)
        let result = dcm_diff_with_metadata(
            &left,
            &right,
            &CalSource::Dcm(PathBuf::from("left.DCM")),
            &CalSource::Dcm(PathBuf::from("right.DCM")),
        );
        assert_eq!(result.differences.len(), 0);
    }

    #[test]
    fn test_dcm_diff_f32_bytes_detects_real_difference() {
        let left = make_dcm_with_constant("param1", 1.0);
        let right = make_dcm_with_constant("param1", 2.0);

        // f32 byte comparison: 1.0 and 2.0 are different even as f32
        let result = dcm_diff_with_metadata(
            &left,
            &right,
            &CalSource::Dcm(PathBuf::from("left.DCM")),
            &CalSource::Dcm(PathBuf::from("right.DCM")),
        );
        assert_eq!(result.differences.len(), 1);
    }

    #[test]
    fn test_dcm_diff_f32_bytes_suppresses_noise_multiple_types() {
        let left = make_dcm_with_constant("param1", 1.0);
        let mut right_blocks = IndexMap::new();
        let c = FESTWERT::from_f64(
            "param1".to_string(),
            1.0 + 1e-9,
            "desc".to_string(),
            "unit".to_string(),
        );
        right_blocks.insert("param1".to_string(), Block::Constant(c));
        let right = DcmData {
            blocks: right_blocks,
            source_path: None,
        };

        // f32 byte comparison: 0 changes (1e-9 is below f32 precision)
        let result = dcm_diff_with_metadata(
            &left,
            &right,
            &CalSource::Dcm(PathBuf::from("left")),
            &CalSource::Dcm(PathBuf::from("right")),
        );
        assert_eq!(result.differences.len(), 0);
    }
}
