use std::path::Path;
use dcm_utils::{DcmData, dcm_diff};

/// Test 1: 一维表（GRUPPENKENNLINIE）的点数不一致应该被检测到
/// VAR_0014: 原始9个点，修改为7个点
#[test]
fn test_diff_detects_1d_table_dim_change() {
    let original = DcmData::new(Path::new("test-dcms/test1.DCM"));
    let modified = DcmData::new(Path::new("test-dcms/test1_1d_dim_changed.DCM"));

    let diff = dcm_diff(&original, &modified);

    // Should detect VAR_0014 as changed
    let var_0014_changed = diff.iter().any(|d| matches!(d, dcm_utils::diff::DcmDiff::Changed { name, .. } if name == "VAR_0014"));
    assert!(var_0014_changed, "Should detect VAR_0014 as changed (1D table dimension changed from 9 to 7)");

    // Verify the old and new values are different
    let var_0014_diff = diff.iter().find(|d| matches!(d, dcm_utils::diff::DcmDiff::Changed { name, .. } if name == "VAR_0014"));
    if let Some(dcm_utils::diff::DcmDiff::Changed { old, new, .. }) = var_0014_diff {
        // Old should have 9 values, new should have 7 values
        let old_len = match old {
            dcm_utils::value::Value::WERT(v) => v.len(),
            _ => 0,
        };
        let new_len = match new {
            dcm_utils::value::Value::WERT(v) => v.len(),
            _ => 0,
        };
        assert_eq!(old_len, 9, "Old VAR_0014 should have 9 values");
        assert_eq!(new_len, 7, "New VAR_0014 should have 7 values");
    }
}

/// Test 2: 二维表（GRUPPENKENNFELD）的点数不一致应该被检测到
/// VAR_0304: 原始16x9，修改为12x9
#[test]
fn test_diff_detects_2d_table_dim_change() {
    let original = DcmData::new(Path::new("test-dcms/test1.DCM"));
    let modified = DcmData::new(Path::new("test-dcms/test1_2d_dim_changed.DCM"));

    let diff = dcm_diff(&original, &modified);

    // Should detect VAR_0304 as changed (ChangedMap because it's a Map type)
    let var_0304_changed = diff.iter().any(|d| matches!(d, dcm_utils::diff::DcmDiff::ChangedMap { name, .. } if name == "VAR_0304"));
    assert!(var_0304_changed, "Should detect VAR_0304 as changed (2D table dimension changed from 16x9 to 12x9)");
}

/// Test 3: 一维表使用的轴点变量名发生改变应该被检测到
/// VAR_0014: 原始*SSTX VAR_0013，修改为*SSTX VAR_0239
#[test]
fn test_diff_detects_axis_var_name_change_for_1d_table() {
    let original = DcmData::new(Path::new("test-dcms/test1.DCM"));
    let modified = DcmData::new(Path::new("test-dcms/test1_axis_changed.DCM"));

    let diff = dcm_diff(&original, &modified);

    // Should detect VAR_0014 as changed because axis_var_name changed from VAR_0013 to VAR_0239
    let var_0014_changed = diff.iter().any(|d| {
        matches!(d, dcm_utils::diff::DcmDiff::Changed { name, .. } if name == "VAR_0014")
    });
    assert!(var_0014_changed, "Should detect VAR_0014 as changed (axis_var_name changed from VAR_0013 to VAR_0239)");
}

/// Test 4: JSON output should contain detailed change information for table dimension changes
#[test]
fn test_diff_json_contains_table_change_details() {
    let original = DcmData::new(Path::new("test-dcms/test1.DCM"));
    let modified = DcmData::new(Path::new("test-dcms/test1_1d_dim_changed.DCM"));

    let diff = dcm_diff(&original, &modified);
    let json_output = serde_json::to_string_pretty(&diff).expect("Failed to serialize diff to JSON");

    // Verify JSON contains the changed variable
    assert!(json_output.contains("VAR_0014"), "JSON should contain VAR_0014");
    assert!(json_output.contains("Changed"), "JSON should contain 'Changed' variant for table changes");

    // Verify old and new values are present
    assert!(json_output.contains("\"old\""), "JSON should contain 'old' field");
    assert!(json_output.contains("\"new\""), "JSON should contain 'new' field");
}

/// Test 5: Verify the specific values are correctly reported for 1D table changes
#[test]
fn test_diff_reports_correct_values_for_1d_table_change() {
    let original = DcmData::new(Path::new("test-dcms/test1.DCM"));
    let modified = DcmData::new(Path::new("test-dcms/test1_1d_dim_changed.DCM"));

    let diff = dcm_diff(&original, &modified);

    let var_0014_diff = diff.iter().find(|d| matches!(d, dcm_utils::diff::DcmDiff::Changed { name, .. } if name == "VAR_0014"));

    if let Some(dcm_utils::diff::DcmDiff::Changed { old, new, .. }) = var_0014_diff {
        // Verify some specific old values
        match old {
            dcm_utils::value::Value::WERT(v) => {
                assert!((v[0] - 320.0).abs() < 0.001, "First old value should be 320.0");
                assert!((v[8] - 1600.0).abs() < 0.001, "Last old value should be 1600.0");
            },
            _ => panic!("Old value should be WERT variant"),
        }

        // Verify new values have 7 elements
        match new {
            dcm_utils::value::Value::WERT(v) => {
                assert_eq!(v.len(), 7, "New values should have 7 elements");
            },
            _ => panic!("New value should be WERT variant"),
        }
    } else {
        panic!("VAR_0014 should be in diff as Changed");
    }
}
