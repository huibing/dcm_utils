use std::path::{Path, PathBuf};
use dcm_utils::{
    DcmData,
    diff::{CalSource, dcm_diff_with_metadata},
    gen::gen_dcm_data,
};

/// DCM vs DCM: compare two known-different test DCM files
#[test]
fn test_diff_dcm_vs_dcm() {
    let left = CalSource::Dcm(PathBuf::from("test-dcms/test1.DCM"));
    let right = CalSource::Dcm(PathBuf::from("test-dcms/test1_modified.DCM"));

    let left_data = left.load().expect("should load test1.DCM");
    let right_data = right.load().expect("should load test1_modified.DCM");

    let result = dcm_diff_with_metadata(&left_data, &right_data, &left, &right);

    assert!(result.summary.total > 0, "Should detect differences between different DCM files");
    assert_eq!(result.metadata.left_label, left.label());
    assert_eq!(result.metadata.right_label, right.label());
}

/// A2L+HEX vs A2L+HEX same: diff identical generated data, expect zero differences
#[test]
fn test_diff_a2l_vs_a2l_same() {
    let a2l = Path::new("test-dcms/simple_test.a2l");
    let hex = Path::new("test-dcms/simple_test.hex");

    let left_data = gen_dcm_data(a2l, hex).expect("should generate DCM data");
    let right_data = gen_dcm_data(a2l, hex).expect("should generate DCM data");

    let left_src = CalSource::A2lHex { a2l: a2l.to_path_buf(), hex: hex.to_path_buf() };
    let right_src = CalSource::A2lHex { a2l: a2l.to_path_buf(), hex: hex.to_path_buf() };

    let result = dcm_diff_with_metadata(&left_data, &right_data, &left_src, &right_src);

    assert_eq!(result.summary.total, 0, "Identical A2L+HEX should produce zero differences");
}

/// A2L+HEX vs A2L+HEX different: compare generated data against a modified copy
#[test]
fn test_diff_a2l_vs_a2l_different() {
    let a2l = Path::new("test-dcms/simple_test.a2l");
    let hex = Path::new("test-dcms/simple_test.hex");

    let left_data = gen_dcm_data(a2l, hex).expect("should generate DCM data");
    let mut right_data = gen_dcm_data(a2l, hex).expect("should generate DCM data");

    // Remove a block from right_data to simulate a different source
    if let Some(first_name) = right_data.get_all_variable_names().first().cloned() {
        right_data.blocks.shift_remove(&first_name);
    }

    let left_src = CalSource::A2lHex { a2l: a2l.to_path_buf(), hex: hex.to_path_buf() };
    let right_src = CalSource::A2lHex { a2l: a2l.to_path_buf(), hex: hex.to_path_buf() };

    let result = dcm_diff_with_metadata(&left_data, &right_data, &left_src, &right_src);

    assert!(result.summary.total > 0, "Different A2L+HEX sources should produce differences");
}

/// A2L+HEX vs DCM round-trip: generate DCM from A2L+HEX, compare against itself
#[test]
fn test_diff_a2l_vs_dcm_roundtrip() {
    let a2l = Path::new("test-dcms/simple_test.a2l");
    let hex = Path::new("test-dcms/simple_test.hex");

    let left_data = gen_dcm_data(a2l, hex).expect("should generate DCM data");

    // Write to temp file with unique name to avoid collision
    let temp_path = format!("output/roundtrip_test_{}.DCM", std::process::id());
    left_data.render_to_file(Path::new(&temp_path));

    let right_data = DcmData::new(Path::new(&temp_path));

    let left_src = CalSource::A2lHex { a2l: a2l.to_path_buf(), hex: hex.to_path_buf() };
    let right_src = CalSource::Dcm(PathBuf::from(&temp_path));

    let result = dcm_diff_with_metadata(&left_data, &right_data, &left_src, &right_src);

    // Clean up temp file
    let _ = std::fs::remove_file(&temp_path);

    assert_eq!(result.summary.total, 0, "Round-trip A2L+HEX→DCM→file→DCM should produce zero differences");
}

/// A2L+HEX vs DCM different: compare generated data against an unrelated DCM file
#[test]
fn test_diff_a2l_vs_dcm_different() {
    let a2l = Path::new("test-dcms/simple_test.a2l");
    let hex = Path::new("test-dcms/simple_test.hex");

    let left_data = gen_dcm_data(a2l, hex).expect("should generate DCM data");
    let right_data = DcmData::new(Path::new("test-dcms/test1.DCM"));

    let left_src = CalSource::A2lHex { a2l: a2l.to_path_buf(), hex: hex.to_path_buf() };
    let right_src = CalSource::Dcm(PathBuf::from("test-dcms/test1.DCM"));

    let result = dcm_diff_with_metadata(&left_data, &right_data, &left_src, &right_src);

    assert!(result.summary.total > 0, "A2L+HEX vs unrelated DCM should produce differences");
}
