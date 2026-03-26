use std::path::Path;
use dcm_utils::{DcmData, dcm_diff};

/// Test that diff correctly identifies all types of changes between two DCM files
///
/// Changes made in test1_modified.DCM:
/// 1. VAR_0617: Modified LANGNAME and all WERT values (0.0 -> 1.0-7.0)
/// 2. VAR_0030: Modified LANGNAME and WERT value (1600.0 -> 2000.0)
/// 3. VAR_NEW_001: Added new FESTWERT variable
/// 4. VAR_0272, VAR_0239, VAR_0243: Deleted (axis variables that differ between files)
/// 5. VAR_0304: Changed (GRUPPENKENNFELD with modified axis)
#[test]
fn test_diff_detects_all_change_types() {
    // Load the original and modified DCM files
    let original = DcmData::new(Path::new("test-dcms/test1.DCM"));
    let modified = DcmData::new(Path::new("test-dcms/test1_modified.DCM"));

    // Calculate diff
    let diff = dcm_diff(&original, &modified);

    // Serialize to JSON to verify JSON output
    let json_output = serde_json::to_string_pretty(&diff).expect("Failed to serialize diff to JSON");

    // Verify JSON output is not empty
    assert!(!json_output.is_empty(), "JSON output should not be empty");
    assert!(json_output.starts_with('['), "JSON output should be an array");

    // Count different types of changes
    let new_count = diff.iter().filter(|d| matches!(d, dcm_utils::diff::DcmDiff::New { .. })).count();
    let deleted_count = diff.iter().filter(|d| matches!(d, dcm_utils::diff::DcmDiff::Deleted { .. })).count();
    let changed_count = diff.iter().filter(|d| matches!(d, dcm_utils::diff::DcmDiff::Changed { .. } | dcm_utils::diff::DcmDiff::ChangedMap { .. })).count();

    // Assert expected counts based on actual diff output
    assert_eq!(new_count, 1, "Should detect exactly 1 new variable (VAR_NEW_001)");
    assert_eq!(deleted_count, 3, "Should detect exactly 3 deleted variables (VAR_0272, VAR_0239, VAR_0243)");
    assert_eq!(changed_count, 3, "Should detect exactly 3 changed variables (VAR_0617, VAR_0030, VAR_0304)");

    // Verify specific changes are detected
    let diff_names: Vec<String> = diff.iter().map(|d| match d {
        dcm_utils::diff::DcmDiff::New { name, .. } => name.clone(),
        dcm_utils::diff::DcmDiff::Deleted { name, .. } => name.clone(),
        dcm_utils::diff::DcmDiff::Changed { name, .. } => name.clone(),
        dcm_utils::diff::DcmDiff::ChangedMap { name, .. } => name.clone(),
    }).collect();

    // Verify new variable
    assert!(diff_names.contains(&"VAR_NEW_001".to_string()), "Should detect VAR_NEW_001 as new");

    // Verify deleted variables
    assert!(diff_names.contains(&"VAR_0272".to_string()), "Should detect VAR_0272 as deleted");
    assert!(diff_names.contains(&"VAR_0239".to_string()), "Should detect VAR_0239 as deleted");
    assert!(diff_names.contains(&"VAR_0243".to_string()), "Should detect VAR_0243 as deleted");

    // Verify changed variables
    assert!(diff_names.contains(&"VAR_0617".to_string()), "Should detect VAR_0617 as changed");
    assert!(diff_names.contains(&"VAR_0030".to_string()), "Should detect VAR_0030 as changed");
    assert!(diff_names.contains(&"VAR_0304".to_string()), "Should detect VAR_0304 as changed");
}

/// Test that JSON output contains expected structure for each diff type
#[test]
fn test_diff_json_structure() {
    let original = DcmData::new(Path::new("test-dcms/test1.DCM"));
    let modified = DcmData::new(Path::new("test-dcms/test1_modified.DCM"));

    let diff = dcm_diff(&original, &modified);
    let json_output = serde_json::to_string_pretty(&diff).expect("Failed to serialize diff to JSON");

    // Verify JSON contains expected fields for "New" type
    assert!(json_output.contains("\"New\""), "JSON should contain 'New' variant");

    // Verify JSON contains expected fields for "Deleted" type
    assert!(json_output.contains("\"Deleted\""), "JSON should contain 'Deleted' variant");

    // Verify JSON contains expected fields for "Changed" type
    assert!(json_output.contains("\"Changed\""), "JSON should contain 'Changed' variant");

    // Verify variable names are in the JSON
    assert!(json_output.contains("VAR_NEW_001"), "JSON should contain VAR_NEW_001");
    assert!(json_output.contains("VAR_0272"), "JSON should contain VAR_0272 (deleted)");
    assert!(json_output.contains("VAR_0239"), "JSON should contain VAR_0239 (deleted)");
    assert!(json_output.contains("VAR_0243"), "JSON should contain VAR_0243 (deleted)");
    assert!(json_output.contains("VAR_0617"), "JSON should contain VAR_0617");
    assert!(json_output.contains("VAR_0030"), "JSON should contain VAR_0030");
    assert!(json_output.contains("VAR_0304"), "JSON should contain VAR_0304 (ChangedMap)");

    // Verify "old" and "new" fields exist for Changed variants
    assert!(json_output.contains("\"old\""), "JSON should contain 'old' field for Changed variants");
    assert!(json_output.contains("\"new\""), "JSON should contain 'new' field for Changed variants");

    // Verify ChangedMap has string values for old and new (serialized JSON)
    assert!(json_output.contains("\"ChangedMap\""), "JSON should contain 'ChangedMap' variant");
}

/// Test that unchanged variables are not included in diff output
#[test]
fn test_unchanged_variables_not_in_diff() {
    let original = DcmData::new(Path::new("test-dcms/test1.DCM"));
    let modified = DcmData::new(Path::new("test-dcms/test1_modified.DCM"));

    let diff = dcm_diff(&original, &modified);

    // These variables should NOT be in diff (they're unchanged)
    let diff_names: Vec<String> = diff.iter().map(|d| match d {
        dcm_utils::diff::DcmDiff::New { name, .. } => name.clone(),
        dcm_utils::diff::DcmDiff::Deleted { name, .. } => name.clone(),
        dcm_utils::diff::DcmDiff::Changed { name, .. } => name.clone(),
        dcm_utils::diff::DcmDiff::ChangedMap { name, .. } => name.clone(),
    }).collect();

    // These variables should be unchanged
    assert!(!diff_names.contains(&"VAR_0618".to_string()), "VAR_0618 should not appear in diff (unchanged)");
    assert!(!diff_names.contains(&"VAR_0014".to_string()), "VAR_0014 should not appear in diff (unchanged)");
    assert!(!diff_names.contains(&"VAR_0013".to_string()), "VAR_0013 should not appear in diff (unchanged)");
    assert!(!diff_names.contains(&"VAR_0670".to_string()), "VAR_0670 should not appear in diff (unchanged)");
}
