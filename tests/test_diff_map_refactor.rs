use std::path::Path;
use dcm_utils::{DcmData, diff::{dcm_diff_with_metadata, DcmDiffResult, DcmDiff}};

/// Test that ChangedMap outputs clean, structured JSON without escaped strings
#[test]
fn test_changedmap_has_structured_output() {
    let original_path = Path::new("test-dcms/test_map_base.DCM");
    let modified_path = Path::new("test-dcms/test_map_modified.DCM");

    let original = DcmData::new(original_path);
    let modified = DcmData::new(modified_path);

    let result = dcm_diff_with_metadata(&original, &modified, original_path, modified_path);

    // Find MAP_TEST_001 change
    let map_diff = result.differences.iter().find_map(|d| {
        match d {
            DcmDiff::ChangedMap { name, detail, .. } => {
                if name == "MAP_TEST_001" {
                    Some(detail)
                } else {
                    None
                }
            }
            _ => None,
        }
    });

    assert!(map_diff.is_some(), "Should find MAP_TEST_001 ChangedMap");
    let detail = map_diff.unwrap();

    // Verify detail has structured fields, not escaped JSON strings
    assert_eq!(detail.old_values.dim_x, 6, "Old X dimension should be 6");
    assert_eq!(detail.old_values.dim_y, 4, "Old Y dimension should be 4");
    assert_eq!(detail.new_values.dim_x, 5, "New X dimension should be 5");
    assert_eq!(detail.new_values.dim_y, 3, "New Y dimension should be 3");

    // Verify axis variable names
    assert_eq!(detail.old_values.x_axis_name, "AXIS_X_001");
    assert_eq!(detail.old_values.y_axis_name, "AXIS_Y_001");
    assert_eq!(detail.new_values.x_axis_name, "AXIS_X_002");
    assert_eq!(detail.new_values.y_axis_name, "AXIS_Y_002");

    // Verify axis point counts
    assert_eq!(detail.old_values.x_axis.len(), 6, "Old X axis should have 6 points");
    assert_eq!(detail.old_values.y_axis.len(), 4, "Old Y axis should have 4 points");
    assert_eq!(detail.new_values.x_axis.len(), 5, "New X axis should have 5 points");
    assert_eq!(detail.new_values.y_axis.len(), 3, "New Y axis should have 3 points");

    // Verify values are accessible
    assert_eq!(detail.old_values.values.len(), 24, "Old values should have 24 elements (6x4)");
    assert_eq!(detail.new_values.values.len(), 15, "New values should have 15 elements (5x3)");
}

/// Test that JSON output doesn't contain escaped strings
#[test]
fn test_changedmap_json_is_clean() {
    let original_path = Path::new("test-dcms/test_map_base.DCM");
    let modified_path = Path::new("test-dcms/test_map_modified.DCM");

    let original = DcmData::new(original_path);
    let modified = DcmData::new(modified_path);

    let result = dcm_diff_with_metadata(&original, &modified, original_path, modified_path);

    let json_output = serde_json::to_string_pretty(&result).expect("Failed to serialize");

    // The JSON should NOT contain escaped newlines (which indicate nested JSON strings)
    assert!(!json_output.contains("\\n"), "JSON should not contain escaped newlines");

    // The JSON should NOT contain escaped quotes (which indicate nested JSON strings)
    assert!(!json_output.contains("\\\""), "JSON should not contain escaped quotes");

    // The JSON should contain dimension fields
    assert!(json_output.contains("\"dim_x\":"), "JSON should have dim_x field");
    assert!(json_output.contains("\"dim_y\":"), "JSON should have dim_y field");
    assert!(json_output.contains("\"values\": ["), "JSON should have values array");
    assert!(json_output.contains("\"x_axis\": ["), "JSON should have x_axis array");
    assert!(json_output.contains("\"y_axis\": ["), "JSON should have y_axis array");
}

/// Test that MapChangeDetail fields are properly populated
#[test]
fn test_map_change_detail_populated() {
    let original_path = Path::new("test-dcms/test_map_base.DCM");
    let modified_path = Path::new("test-dcms/test_map_modified.DCM");

    let original = DcmData::new(original_path);
    let modified = DcmData::new(modified_path);

    let result = dcm_diff_with_metadata(&original, &modified, original_path, modified_path);

    let map_diff = result.differences.iter().find_map(|d| {
        match d {
            DcmDiff::ChangedMap { name, detail, .. } if name == "MAP_TEST_001" => Some(detail),
            _ => None,
        }
    }).expect("Should find MAP_TEST_001");

    // Verify all fields are populated
    assert!(!map_diff.old_values.x_axis_name.is_empty(), "Old X axis name should be populated");
    assert!(!map_diff.old_values.y_axis_name.is_empty(), "Old Y axis name should be populated");
    assert!(!map_diff.new_values.x_axis_name.is_empty(), "New X axis name should be populated");
    assert!(!map_diff.new_values.y_axis_name.is_empty(), "New Y axis name should be populated");

    // Verify attrs exist
    assert!(!map_diff.old_values.attrs.is_empty(), "Old attrs should be populated");
    assert!(!map_diff.new_values.attrs.is_empty(), "New attrs should be populated");

    // Check for LANGNAME change
    let old_langname = map_diff.old_values.attrs.iter()
        .find(|a| a.identifier == "LANGNAME")
        .map(|a| a.value.clone());
    let new_langname = map_diff.new_values.attrs.iter()
        .find(|a| a.identifier == "LANGNAME")
        .map(|a| a.value.clone());

    assert_eq!(old_langname, Some("Base 2D calibration map".to_string()));
    assert_eq!(new_langname, Some("Modified 2D calibration map".to_string()));
}

/// Test that ChangedMap can be roundtripped without data loss
#[test]
fn test_changedmap_roundtrip() {
    let original_path = Path::new("test-dcms/test_map_base.DCM");
    let modified_path = Path::new("test-dcms/test_map_modified.DCM");

    let original = DcmData::new(original_path);
    let modified = DcmData::new(modified_path);

    let result = dcm_diff_with_metadata(&original, &modified, original_path, modified_path);

    // Serialize
    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize");

    // Deserialize
    let deserialized: DcmDiffResult = serde_json::from_str(&json).expect("Failed to deserialize");

    // Find MAP_TEST_001 in deserialized result
    let map_diff = deserialized.differences.iter().find_map(|d| {
        match d {
            DcmDiff::ChangedMap { name, detail, .. } if name == "MAP_TEST_001" => Some(detail),
            _ => None,
        }
    }).expect("Should find MAP_TEST_001 after roundtrip");

    // Verify data is preserved
    assert_eq!(map_diff.old_values.dim_x, 6);
    assert_eq!(map_diff.old_values.dim_y, 4);
    assert_eq!(map_diff.new_values.dim_x, 5);
    assert_eq!(map_diff.new_values.dim_y, 3);

    // Verify values are preserved
    assert_eq!(map_diff.old_values.values.len(), 24);
    assert_eq!(map_diff.new_values.values.len(), 15);
}

/// Test that 2D array values are accessible
#[test]
fn test_2d_array_values_accessible() {
    let original_path = Path::new("test-dcms/test_map_base.DCM");
    let modified_path = Path::new("test-dcms/test_map_modified.DCM");

    let original = DcmData::new(original_path);
    let modified = DcmData::new(modified_path);

    let result = dcm_diff_with_metadata(&original, &modified, original_path, modified_path);

    let map_diff = result.differences.iter().find_map(|d| {
        match d {
            DcmDiff::ChangedMap { name, detail, .. } if name == "MAP_TEST_001" => Some(detail),
            _ => None,
        }
    }).expect("Should find MAP_TEST_001");

    // Verify values_2d is accessible if stored
    if let Some(ref values_2d) = map_diff.old_values.values_2d {
        assert_eq!(values_2d.len(), 4, "Old 2D values should have 4 rows");
        assert_eq!(values_2d[0].len(), 6, "Old 2D values first row should have 6 columns");
    }

    if let Some(ref values_2d) = map_diff.new_values.values_2d {
        assert_eq!(values_2d.len(), 3, "New 2D values should have 3 rows");
        assert_eq!(values_2d[0].len(), 5, "New 2D values first row should have 5 columns");
    }
}
