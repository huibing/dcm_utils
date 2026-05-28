use dcm_utils::{
    diff::{dcm_diff_with_metadata, CalSource, DcmDiff, DcmDiffResult},
    DcmData,
};
use std::path::Path;

/// Test that diff result contains file metadata information
#[test]
fn test_diff_contains_file_metadata() {
    let original_path = Path::new("test-dcms/test1.DCM");
    let modified_path = Path::new("test-dcms/test1_modified.DCM");

    let original = DcmData::new(original_path);
    let modified = DcmData::new(modified_path);

    let result = dcm_diff_with_metadata(
        &original,
        &modified,
        &CalSource::Dcm(original_path.to_path_buf()),
        &CalSource::Dcm(modified_path.to_path_buf()),
        false,
    );

    // Verify metadata contains file paths
    assert_eq!(
        result.metadata.left_label, "test-dcms/test1.DCM",
        "Should contain original file path"
    );
    assert_eq!(
        result.metadata.right_label, "test-dcms/test1_modified.DCM",
        "Should contain modified file path"
    );

    // Verify metadata contains timestamp
    assert!(
        !result.metadata.timestamp.is_empty(),
        "Should contain timestamp"
    );
}

/// Test that JSON output includes file metadata
#[test]
fn test_diff_json_contains_file_info() {
    let original_path = Path::new("test-dcms/test1.DCM");
    let modified_path = Path::new("test-dcms/test1_modified.DCM");

    let original = DcmData::new(original_path);
    let modified = DcmData::new(modified_path);

    let result = dcm_diff_with_metadata(
        &original,
        &modified,
        &CalSource::Dcm(original_path.to_path_buf()),
        &CalSource::Dcm(modified_path.to_path_buf()),
        false,
    );

    let json_output =
        serde_json::to_string_pretty(&result).expect("Failed to serialize diff result to JSON");

    // Verify JSON contains file information
    assert!(
        json_output.contains("metadata"),
        "JSON should contain metadata field"
    );
    assert!(
        json_output.contains("original_file"),
        "JSON should contain original_file field"
    );
    assert!(
        json_output.contains("modified_file"),
        "JSON should contain modified_file field"
    );
    assert!(
        json_output.contains("timestamp"),
        "JSON should contain timestamp field"
    );
    assert!(
        json_output.contains("test-dcms/test1.DCM"),
        "JSON should contain original file path value"
    );
    assert!(
        json_output.contains("test-dcms/test1_modified.DCM"),
        "JSON should contain modified file path value"
    );
}

/// Test that detailed change info includes block types and descriptions
#[test]
fn test_diff_contains_detailed_change_info() {
    let original_path = Path::new("test-dcms/test1.DCM");
    let modified_path = Path::new("test-dcms/test1_modified.DCM");

    let original = DcmData::new(original_path);
    let modified = DcmData::new(modified_path);

    let result = dcm_diff_with_metadata(
        &original,
        &modified,
        &CalSource::Dcm(original_path.to_path_buf()),
        &CalSource::Dcm(modified_path.to_path_buf()),
        false,
    );

    // Verify summary counts
    assert!(
        result.summary.new_count > 0
            || result.summary.deleted_count > 0
            || result.summary.changed_count > 0,
        "Summary should have non-zero counts when there are differences"
    );

    // Verify total matches sum of changes
    let expected_total =
        result.summary.new_count + result.summary.deleted_count + result.summary.changed_count;
    assert_eq!(
        result.summary.total, expected_total,
        "Total should equal sum of new + deleted + changed"
    );
}

/// Test that Changed variant includes detailed info about what changed
#[test]
fn test_diff_changed_includes_description() {
    let original_path = Path::new("test-dcms/test1.DCM");
    let modified_path = Path::new("test-dcms/test1_1d_dim_changed.DCM");

    let original = DcmData::new(original_path);
    let modified = DcmData::new(modified_path);

    let result = dcm_diff_with_metadata(
        &original,
        &modified,
        &CalSource::Dcm(original_path.to_path_buf()),
        &CalSource::Dcm(modified_path.to_path_buf()),
        false,
    );

    // Find VAR_0014 change
    let var_0014_diff = result.differences.iter().find(|d| match d {
        DcmDiff::Changed { name, .. } => name == "VAR_0014",
        _ => false,
    });

    if let Some(DcmDiff::Changed {
        name, description, ..
    }) = var_0014_diff
    {
        assert_eq!(name, "VAR_0014");
        // Description should indicate what changed
        assert!(
            description.is_some(),
            "Changed diff should include description"
        );
        assert!(
            !description.as_ref().unwrap().is_empty(),
            "Description should not be empty"
        );
    } else {
        panic!("VAR_0014 should be in diff as Changed");
    }
}

/// Test that DcmDiffResult can be serialized and deserialized
#[test]
fn test_diff_result_roundtrip() {
    let original_path = Path::new("test-dcms/test1.DCM");
    let modified_path = Path::new("test-dcms/test1_modified.DCM");

    let original = DcmData::new(original_path);
    let modified = DcmData::new(modified_path);

    let result = dcm_diff_with_metadata(
        &original,
        &modified,
        &CalSource::Dcm(original_path.to_path_buf()),
        &CalSource::Dcm(modified_path.to_path_buf()),
        false,
    );

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&result).expect("Failed to serialize");

    // Deserialize back
    let deserialized: DcmDiffResult = serde_json::from_str(&json).expect("Failed to deserialize");

    // Verify fields match
    assert_eq!(deserialized.metadata.left_label, result.metadata.left_label);
    assert_eq!(
        deserialized.metadata.right_label,
        result.metadata.right_label
    );
    assert_eq!(deserialized.differences.len(), result.differences.len());
    assert_eq!(deserialized.summary.total, result.summary.total);
}

/// Test empty diff result (comparing identical files)
#[test]
fn test_diff_empty_result_for_identical_files() {
    let original_path = Path::new("test-dcms/test1.DCM");
    let modified_path = Path::new("test-dcms/test1.DCM");

    let original = DcmData::new(original_path);
    let modified = DcmData::new(modified_path);

    let result = dcm_diff_with_metadata(
        &original,
        &modified,
        &CalSource::Dcm(original_path.to_path_buf()),
        &CalSource::Dcm(modified_path.to_path_buf()),
        false,
    );

    // Should have no differences
    assert!(
        result.differences.is_empty(),
        "Identical files should have no differences"
    );
    assert_eq!(
        result.summary.total, 0,
        "Total should be 0 for identical files"
    );
    assert_eq!(result.summary.new_count, 0);
    assert_eq!(result.summary.deleted_count, 0);
    assert_eq!(result.summary.changed_count, 0);
}

/// Test summary counts are accurate
#[test]
fn test_diff_summary_counts_are_accurate() {
    let original_path = Path::new("test-dcms/test1.DCM");
    let modified_path = Path::new("test-dcms/test1_modified.DCM");

    let original = DcmData::new(original_path);
    let modified = DcmData::new(modified_path);

    let result = dcm_diff_with_metadata(
        &original,
        &modified,
        &CalSource::Dcm(original_path.to_path_buf()),
        &CalSource::Dcm(modified_path.to_path_buf()),
        false,
    );

    // Count each type manually
    let actual_new = result
        .differences
        .iter()
        .filter(|d| matches!(d, DcmDiff::New { .. }))
        .count();
    let actual_deleted = result
        .differences
        .iter()
        .filter(|d| matches!(d, DcmDiff::Deleted { .. }))
        .count();
    let actual_changed = result
        .differences
        .iter()
        .filter(|d| matches!(d, DcmDiff::Changed { .. } | DcmDiff::ChangedMap { .. }))
        .count();

    assert_eq!(
        result.summary.new_count, actual_new,
        "New count should match actual new diffs"
    );
    assert_eq!(
        result.summary.deleted_count, actual_deleted,
        "Deleted count should match actual deleted diffs"
    );
    assert_eq!(
        result.summary.changed_count, actual_changed,
        "Changed count should match actual changed diffs"
    );
}
