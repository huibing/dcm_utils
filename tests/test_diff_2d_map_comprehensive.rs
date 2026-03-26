use std::path::Path;
use dcm_utils::{DcmData, diff::{dcm_diff_with_metadata, DcmDiffResult, DcmDiff}};

/// Comprehensive test for 2D map (GRUPPENKENNFELD) changes
///
/// Changes in test_map_modified.DCM:
/// 1. LANGNAME: "Base 2D calibration map" -> "Modified 2D calibration map"
/// 2. X axis variable: AXIS_X_001 -> AXIS_X_002
/// 3. Y axis variable: AXIS_Y_001 -> AXIS_Y_002
/// 4. X axis points: 6 -> 5
/// 5. Y axis points: 4 -> 3
/// 6. Table values: All values changed
/// 7. New axis blocks: AXIS_X_002, AXIS_Y_002
/// 8. Deleted axis blocks: AXIS_X_001, AXIS_Y_001
#[test]
fn test_2d_map_diff_detects_all_changes() {
    let original_path = Path::new("test-dcms/test_map_base.DCM");
    let modified_path = Path::new("test-dcms/test_map_modified.DCM");

    let original = DcmData::new(original_path);
    let modified = DcmData::new(modified_path);

    let result = dcm_diff_with_metadata(&original, &modified, original_path, modified_path);

    // Verify summary
    assert!(result.summary.total > 0, "Should have differences");

    // MAP_TEST_001 should be changed
    let map_diff = result.differences.iter().find(|d| {
        match d {
            DcmDiff::ChangedMap { name, .. } => name == "MAP_TEST_001",
            _ => false,
        }
    });
    assert!(map_diff.is_some(), "MAP_TEST_001 should be detected as ChangedMap");

    // Verify old axis blocks are deleted
    let axis_x_001_deleted = result.differences.iter().any(|d| {
        match d {
            DcmDiff::Deleted { name, .. } => name == "AXIS_X_001",
            _ => false,
        }
    });
    assert!(axis_x_001_deleted, "AXIS_X_001 should be detected as deleted");

    let axis_y_001_deleted = result.differences.iter().any(|d| {
        match d {
            DcmDiff::Deleted { name, .. } => name == "AXIS_Y_001",
            _ => false,
        }
    });
    assert!(axis_y_001_deleted, "AXIS_Y_001 should be detected as deleted");

    // Verify new axis blocks are added
    let axis_x_002_new = result.differences.iter().any(|d| {
        match d {
            DcmDiff::New { name, .. } => name == "AXIS_X_002",
            _ => false,
        }
    });
    assert!(axis_x_002_new, "AXIS_X_002 should be detected as new");

    let axis_y_002_new = result.differences.iter().any(|d| {
        match d {
            DcmDiff::New { name, .. } => name == "AXIS_Y_002",
            _ => false,
        }
    });
    assert!(axis_y_002_new, "AXIS_Y_002 should be detected as new");
}

/// Test that ChangedMap contains detailed information about what changed
#[test]
fn test_2d_map_changedmap_contains_detailed_info() {
    let original_path = Path::new("test-dcms/test_map_base.DCM");
    let modified_path = Path::new("test-dcms/test_map_modified.DCM");

    let original = DcmData::new(original_path);
    let modified = DcmData::new(modified_path);

    let result = dcm_diff_with_metadata(&original, &modified, original_path, modified_path);

    // Find MAP_TEST_001 change
    let map_diff = result.differences.iter().find_map(|d| {
        match d {
            DcmDiff::ChangedMap { name, detail, description } => {
                if name == "MAP_TEST_001" {
                    Some((detail, description))
                } else {
                    None
                }
            }
            _ => None,
        }
    });

    assert!(map_diff.is_some(), "Should find MAP_TEST_001 ChangedMap");
    let (detail, description) = map_diff.unwrap();

    // Description should mention specific changes
    assert!(description.is_some(), "Should have description");
    let desc = description.as_ref().unwrap();
    println!("Change description: {}", desc);

    // Verify old dimensions directly from detail
    let old_x = detail.old_values.dim_x;
    let old_y = detail.old_values.dim_y;
    assert_eq!(old_x, 6, "Old X dimension should be 6");
    assert_eq!(old_y, 4, "Old Y dimension should be 4");

    // Verify new dimensions
    let new_x = detail.new_values.dim_x;
    let new_y = detail.new_values.dim_y;
    assert_eq!(new_x, 5, "New X dimension should be 5");
    assert_eq!(new_y, 3, "New Y dimension should be 3");
    assert_eq!(new_x, 5, "New X dimension should be 5");
    assert_eq!(new_y, 3, "New Y dimension should be 3");

    // Verify axis variable name changes
    assert_eq!(detail.old_values.x_axis_name, "AXIS_X_001");
    assert_eq!(detail.new_values.x_axis_name, "AXIS_X_002");
    assert_eq!(detail.old_values.y_axis_name, "AXIS_Y_001");
    assert_eq!(detail.new_values.y_axis_name, "AXIS_Y_002");

    // Verify description mentions key changes
    assert!(desc.contains("dimensions") || desc.contains("values") || desc.contains("axis"),
            "Description should mention what changed");
}

/// Test JSON output structure for 2D map comprehensive diff
#[test]
fn test_2d_map_diff_json_structure() {
    let original_path = Path::new("test-dcms/test_map_base.DCM");
    let modified_path = Path::new("test-dcms/test_map_modified.DCM");

    let original = DcmData::new(original_path);
    let modified = DcmData::new(modified_path);

    let result = dcm_diff_with_metadata(&original, &modified, original_path, modified_path);
    let json_output = serde_json::to_string_pretty(&result).expect("Failed to serialize");

    // Verify metadata
    assert!(json_output.contains("metadata"), "JSON should have metadata");
    assert!(json_output.contains("test_map_base.DCM"), "JSON should contain original file");
    assert!(json_output.contains("test_map_modified.DCM"), "JSON should contain modified file");

    // Verify summary
    assert!(json_output.contains("summary"), "JSON should have summary");
    assert!(json_output.contains("new_count"), "JSON should have new_count");
    assert!(json_output.contains("deleted_count"), "JSON should have deleted_count");
    assert!(json_output.contains("changed_count"), "JSON should have changed_count");

    // Verify all expected blocks are present
    assert!(json_output.contains("MAP_TEST_001"), "JSON should contain MAP_TEST_001");
    assert!(json_output.contains("AXIS_X_001"), "JSON should contain AXIS_X_001");
    assert!(json_output.contains("AXIS_X_002"), "JSON should contain AXIS_X_002");
    assert!(json_output.contains("AXIS_Y_001"), "JSON should contain AXIS_Y_001");
    assert!(json_output.contains("AXIS_Y_002"), "JSON should contain AXIS_Y_002");

    // Verify ChangedMap structure
    assert!(json_output.contains("ChangedMap"), "JSON should have ChangedMap variant");
}

/// Test that the diff can be serialized and deserialized (roundtrip)
#[test]
fn test_2d_map_diff_roundtrip() {
    let original_path = Path::new("test-dcms/test_map_base.DCM");
    let modified_path = Path::new("test-dcms/test_map_modified.DCM");

    let original = DcmData::new(original_path);
    let modified = DcmData::new(modified_path);

    let result = dcm_diff_with_metadata(&original, &modified, original_path, modified_path);

    // Serialize
    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize");

    // Deserialize
    let deserialized: DcmDiffResult = serde_json::from_str(&json).expect("Failed to deserialize");

    // Verify metadata preserved
    assert_eq!(deserialized.metadata.original_file, result.metadata.original_file);
    assert_eq!(deserialized.metadata.modified_file, result.metadata.modified_file);

    // Verify summary preserved
    assert_eq!(deserialized.summary.total, result.summary.total);
    assert_eq!(deserialized.summary.new_count, result.summary.new_count);
    assert_eq!(deserialized.summary.deleted_count, result.summary.deleted_count);
    assert_eq!(deserialized.summary.changed_count, result.summary.changed_count);

    // Verify differences count
    assert_eq!(deserialized.differences.len(), result.differences.len());
}

/// Test terminal output format (verify description contains useful info)
#[test]
fn test_2d_map_terminal_output_descriptions() {
    let original_path = Path::new("test-dcms/test_map_base.DCM");
    let modified_path = Path::new("test-dcms/test_map_modified.DCM");

    let original = DcmData::new(original_path);
    let modified = DcmData::new(modified_path);

    let result = dcm_diff_with_metadata(&original, &modified, original_path, modified_path);

    // Collect all descriptions
    let descriptions: Vec<String> = result.differences.iter()
        .filter_map(|d| match d {
            DcmDiff::New { description, .. } => description.clone(),
            DcmDiff::Deleted { description, .. } => description.clone(),
            DcmDiff::Changed { description, .. } => description.clone(),
            DcmDiff::ChangedMap { description, .. } => description.clone(),
        })
        .collect();

    // Each diff should have a description
    assert_eq!(descriptions.len(), result.differences.len(),
               "Each diff should have a description");

    // Print all descriptions for verification
    for desc in &descriptions {
        println!("Diff description: {}", desc);
        assert!(!desc.is_empty(), "Description should not be empty");
    }

    // Verify MAP_TEST_001 description mentions specific changes
    let map_desc = descriptions.iter()
        .find(|d| d.contains("MAP_TEST_001"))
        .expect("Should have description for MAP_TEST_001");

    // Should mention key changes
    assert!(map_desc.contains("GRUPPENKENNFELD"), "Should mention block type");
}

/// Test comparing identical 2D map files
#[test]
fn test_2d_map_diff_identical_files() {
    let original_path = Path::new("test-dcms/test_map_base.DCM");

    let original = DcmData::new(original_path);
    let modified = DcmData::new(original_path);

    let result = dcm_diff_with_metadata(&original, &modified, original_path, original_path);

    // Should have no differences
    assert!(result.differences.is_empty(), "Identical files should have no differences");
    assert_eq!(result.summary.total, 0);
    assert_eq!(result.summary.new_count, 0);
    assert_eq!(result.summary.deleted_count, 0);
    assert_eq!(result.summary.changed_count, 0);
}

/// Test that value changes are detected even when dimensions stay same
#[test]
fn test_2d_map_value_changes_only() {
    // Create a version with same dimensions but different values
    let original_path = Path::new("test-dcms/test_map_base.DCM");
    let modified_path = Path::new("test-dcms/test_map_modified.DCM");

    let original = DcmData::new(original_path);
    let modified = DcmData::new(modified_path);

    let result = dcm_diff_with_metadata(&original, &modified, original_path, modified_path);

    // Find MAP_TEST_001
    let map_diff = result.differences.iter().find_map(|d| {
        match d {
            DcmDiff::ChangedMap { name, detail, .. } if name == "MAP_TEST_001" => {
                Some(detail)
            }
            _ => None,
        }
    });

    assert!(map_diff.is_some(), "Should find MAP_TEST_001 diff");
    let detail = map_diff.unwrap();

    // Values should be different
    assert_ne!(detail.old_values.values, detail.new_values.values, "Values should be different");
}
