# Diff Multi-Source Comparison Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend the `diff` CLI command to compare calibration values across DCM files and A2L+HEX pairs by normalizing both to `DcmData`.

**Architecture:** Add a `CalSource` enum in `diff.rs` that dispatches loading to either `DcmData::new()` or `gen::gen_dcm_data()`, a `validate_and_build_sources()` function for CLI arg validation, and update `DiffMetadata` with renamed fields + `#[serde(rename)]` for JSON backward compatibility. The core `dcm_diff()` engine is untouched.

**Tech Stack:** Rust, clap derive API, serde, existing `dcm_utils` crate

---

### Task 1: Add CalSource, validation, and update DiffMetadata in diff.rs

**Files:**
- Modify: `src/diff.rs:1-249`

- [ ] **Step 1: Add `CalSource` enum and `use crate::gen::gen_dcm_data` import**

Add at the top of `src/diff.rs` after existing imports:
```rust
use crate::gen::gen_dcm_data;

#[derive(Debug, Clone)]
pub enum CalSource {
    Dcm(PathBuf),
    A2lHex { a2l: PathBuf, hex: PathBuf },
}
```

- [ ] **Step 2: Implement `CalSource::load()`**

```rust
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
            CalSource::A2lHex { a2l, hex } =>
                format!("{} + {}", a2l.display(), hex.display()),
        }
    }
}
```

- [ ] **Step 3: Update `DiffMetadata` struct — rename fields with `#[serde(rename)]`**

Replace lines 13-18 in `src/diff.rs`:
```rust
pub struct DiffMetadata {
    #[serde(rename = "original_file")]
    pub left_label: String,
    #[serde(rename = "modified_file")]
    pub right_label: String,
    pub timestamp: String,
}
```

- [ ] **Step 4: Update `DiffMetadata::new()` to accept `&str` labels**

Replace lines 20-29:
```rust
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
```

- [ ] **Step 5: Update `dcm_diff_with_metadata` signature to accept `&CalSource`**

Replace the function at line 245:
```rust
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
```

- [ ] **Step 6: Add `validate_and_build_sources` function**

Add after `CalSource` impl block:
```rust
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
    for path in dcm {
        sources.push(CalSource::Dcm(path.clone()));
    }
    for (a, h) in a2l.iter().zip(hex.iter()) {
        sources.push(CalSource::A2lHex { a2l: a.clone(), hex: h.clone() });
    }
    let right = sources.pop().unwrap();
    let left = sources.pop().unwrap();
    Ok((left, right))
}
```

- [ ] **Step 7: Verify `diff.rs` compiles**

Run: `cargo build 2>&1`
Expected: Compile errors in `main.rs` and test files (they still reference old `original_file`/`modified_file` fields and old `dcm_diff_with_metadata` signature). `diff.rs` itself should have no errors.

- [ ] **Step 8: Commit**

```bash
git add src/diff.rs
git commit -m "refactor(diff): add CalSource enum, validation, rename DiffMetadata fields"
```

---

### Task 2: Update lib.rs exports

**Files:**
- Modify: `src/lib.rs:8`

- [ ] **Step 1: Add CalSource and validate_and_build_sources to re-exports**

Change line 8 from:
```rust
pub use diff::{DcmDiff, DcmDiffResult, DiffMetadata, DiffSummary, MapChangeDetail, MapValues, MapAttr, dcm_diff, dcm_diff_with_metadata};
```
to:
```rust
pub use diff::{DcmDiff, DcmDiffResult, DiffMetadata, DiffSummary, MapChangeDetail, MapValues, MapAttr, CalSource, dcm_diff, dcm_diff_with_metadata, validate_and_build_sources};
```

- [ ] **Step 2: Verify**

Run: `cargo build 2>&1`
Expected: Still errors in main.rs and tests (not yet updated), but lib.rs export is correct.

- [ ] **Step 3: Commit**

```bash
git add src/lib.rs
git commit -m "refactor: export CalSource and validate_and_build_sources from lib.rs"
```

---

### Task 3: Update main.rs CLI and match arm

**Files:**
- Modify: `src/main.rs:94-247`

- [ ] **Step 1: Replace the `Diff` variant in the `Commands` enum**

Replace lines 94-120 (old `Diff` variant with positional args):
```rust
    /// Compare calibration data from two sources (DCM files or A2L+HEX pairs)
    ///
    /// Generates a detailed comparison showing new, deleted, and changed variables.
    /// Results are printed to console and saved as JSON.
    ///
    /// ## Examples
    ///
    /// Compare two DCM files:
    ///
    ///     dcm_utils diff --dcm original.DCM --dcm modified.DCM
    ///
    /// Compare A2L+HEX against a DCM reference:
    ///
    ///     dcm_utils diff --dcm ref.DCM --a2l cal.a2l -x flash.hex
    ///
    /// Compare two A2L+HEX calibration sets:
    ///
    ///     dcm_utils diff --a2l v1.a2l -x v1.hex --a2l v2.a2l -x v2.hex
    ///
    /// Review the JSON output for detailed changes:
    ///
    ///     cat diff.json | jq '.[] | select(.Changed)'
    Diff {
        /// DCM file source (repeatable, one per side)
        #[arg(long)]
        dcm: Vec<PathBuf>,
        /// A2L calibration description (paired with --hex on the same side)
        #[arg(long)]
        a2l: Vec<PathBuf>,
        /// Intel HEX flash image (paired with --a2l on the same side)
        #[arg(short = 'x', long)]
        hex: Vec<PathBuf>,
        /// Output JSON file for diff results
        #[arg(short, long, default_value = "diff.json")]
        output: PathBuf,
    },
```

- [ ] **Step 2: Update the `Commands::Diff` match arm (lines 192-239)**

Replace:
```rust
        Commands::Diff { original, modified, output } => {
            let original_dcm = DcmData::new(&original);
            let modified_dcm = DcmData::new(&modified);

            let result = dcm_diff_with_metadata(&original_dcm, &modified_dcm, &original, &modified);

            println!("{}", "=== DCM Diff Results ===".bold());
            println!("Original: {}", result.metadata.original_file.cyan());
            println!("Modified: {}", result.metadata.modified_file.cyan());
            // ... rest unchanged ...
```

with:
```rust
        Commands::Diff { dcm, a2l, hex, output } => {
            let (left_src, right_src) = validate_and_build_sources(&dcm, &a2l, &hex)
                .unwrap_or_else(|e| {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                });

            let left_data = left_src.load()
                .unwrap_or_else(|e| {
                    eprintln!("Error loading {}: {}", left_src.label(), e);
                    std::process::exit(1);
                });
            let right_data = right_src.load()
                .unwrap_or_else(|e| {
                    eprintln!("Error loading {}: {}", right_src.label(), e);
                    std::process::exit(1);
                });

            let result = dcm_diff_with_metadata(&left_data, &right_data, &left_src, &right_src);

            // Print summary
            println!("{}", "=== Calibration Diff Results ===".bold());
            println!("Left:  {}", result.metadata.left_label.cyan());
            println!("Right: {}", result.metadata.right_label.cyan());
            println!("Timestamp: {}\n", result.metadata.timestamp.dimmed());

            println!("New blocks: {}", result.summary.new_count.to_string().green());
            println!("Deleted blocks: {}", result.summary.deleted_count.to_string().red());
            println!("Changed blocks: {}", result.summary.changed_count.to_string().yellow());
            println!("Total differences: {}\n", result.summary.total.to_string().bold());

            // Print detailed differences to terminal (unchanged from original)
            if !result.differences.is_empty() {
                println!("{}", "=== Detailed Differences ===".bold());
                for diff in &result.differences {
                    match diff {
                        DcmDiff::New { name, description } => {
                            println!("{} {}: {}", "[NEW]".green().bold(), name.green(),
                                description.as_ref().unwrap_or(&"".to_string()));
                        }
                        DcmDiff::Deleted { name, description } => {
                            println!("{} {}: {}", "[DEL]".red().bold(), name.red(),
                                description.as_ref().unwrap_or(&"".to_string()));
                        }
                        DcmDiff::Changed { name, description, .. } => {
                            println!("{} {}: {}", "[CHG]".yellow().bold(), name.yellow(),
                                description.as_ref().unwrap_or(&"values changed".to_string()));
                        }
                        DcmDiff::ChangedMap { name, description, .. } => {
                            println!("{} {}: {}", "[CHG]".yellow().bold(), name.yellow(),
                                description.as_ref().unwrap_or(&"map changed".to_string()));
                        }
                    }
                }
                println!();
            }

            // Write diff result to JSON file
            let json = serde_json::to_string_pretty(&result).unwrap();
            std::fs::write(&output, json).expect("Failed to write diff output");
            println!("Diff details written to: {}", output.display().to_string().blue());
        },
```

- [ ] **Step 3: Verify build**

Run: `cargo build 2>&1`
Expected: Only test file compile errors remain (they still use old function signature and field names).

- [ ] **Step 4: Commit**

```bash
git add src/main.rs
git commit -m "feat(diff): replace positional args with flag-based multi-source CLI"
```

---

### Task 4: Update existing integration tests

**Files:**
- Modify: `tests/test_diff_enhanced_output.rs`
- Modify: `tests/test_diff_2d_map_comprehensive.rs`
- Modify: `tests/test_diff_map_refactor.rs`

- [ ] **Step 1: Update test_diff_map_refactor.rs**

This file has 5 calls to `dcm_diff_with_metadata`. For each call at lines 13, 64, 91, 131, 167:
- Add `use dcm_utils::diff::CalSource;` to imports
- Replace `dcm_diff_with_metadata(&original, &modified, original_path, modified_path)` with `dcm_diff_with_metadata(&original, &modified, &CalSource::Dcm(original_path.to_path_buf()), &CalSource::Dcm(modified_path.to_path_buf()))`

- [ ] **Step 2: Update test_diff_enhanced_output.rs**

This file has 7 calls. Same pattern as Step 1. Additionally update field access:
- Line 16: `.original_file` → `.left_label`
- Line 17: `.modified_file` → `.right_label`
- Line 38-39: JSON string assertions on `"original_file"`/`"modified_file"` — **unchanged** (serde rename preserves them)
- Lines 112-113: `.original_file` → `.left_label`, `.modified_file` → `.right_label`

- [ ] **Step 3: Update test_diff_2d_map_comprehensive.rs**

This file has 7 calls. Same pattern as Step 1. Additionally update field access:
- Lines 182-183: `.original_file` → `.left_label`, `.modified_file` → `.right_label`

- [ ] **Step 4: Verify build and run existing tests**

Run: `cargo build 2>&1`
Expected: Clean build, no errors.

Run: `cargo test -- test_diff 2>&1`
Expected: All existing diff tests pass.

- [ ] **Step 5: Commit**

```bash
git add tests/test_diff_map_refactor.rs tests/test_diff_enhanced_output.rs tests/test_diff_2d_map_comprehensive.rs
git commit -m "test: update integration tests for CalSource and renamed fields"
```

---

### Task 5: Add unit tests in diff.rs

**Files:**
- Modify: `src/diff.rs` (append test module)

- [ ] **Step 1: Add test module for CalSource**

Append at the bottom of `src/diff.rs`, after the existing functions:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_cal_source_label_dcm() {
        let src = CalSource::Dcm(PathBuf::from("test-dcms/example.DCM"));
        let label = src.label();
        // On Windows, display() uses backslashes; on Unix, forward slashes.
        // Just verify it contains the path.
        assert!(label.contains("example.DCM"), "Label should contain the file name, got: {}", label);
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
        assert!(label.contains(" + "), "Label should contain ' + ' separator");
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
        // DCM entries come first
        assert!(matches!(left, CalSource::Dcm(_)));
        assert!(matches!(right, CalSource::A2lHex { .. }));
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
}
```

- [ ] **Step 2: Run the new unit tests**

Run: `cargo test -- test_cal_source_label test_validate`
Expected: All 9 new tests pass.

- [ ] **Step 3: Commit**

```bash
git add src/diff.rs
git commit -m "test: add unit tests for CalSource and validate_and_build_sources"
```

---

### Task 6: Add new integration tests

**Files:**
- Create: `tests/test_diff_multi_source.rs`

- [ ] **Step 1: Write the integration test file**

```rust
use dcm_utils::DcmData;
use dcm_utils::diff::{dcm_diff_with_metadata, CalSource, validate_and_build_sources};
use std::path::PathBuf;

#[test]
fn test_diff_dcm_vs_dcm_flags() {
    let left_src = CalSource::Dcm(PathBuf::from("test-dcms/test1.DCM"));
    let right_src = CalSource::Dcm(PathBuf::from("test-dcms/test1_modified.DCM"));

    let left = left_src.load().expect("load left DCM");
    let right = right_src.load().expect("load right DCM");

    let result = dcm_diff_with_metadata(&left, &right, &left_src, &right_src);

    assert!(result.summary.total > 0, "Should detect differences between different DCMs");
    assert!(result.metadata.left_label.contains("test1.DCM"));
    assert!(result.metadata.right_label.contains("test1_modified.DCM"));
}

#[test]
fn test_diff_a2l_vs_dcm_roundtrip() {
    let a2l_path = PathBuf::from("test-dcms/simple_test.a2l");
    let hex_path = PathBuf::from("test-dcms/simple_test.hex");

    let a2l_hex_src = CalSource::A2lHex { a2l: a2l_path, hex: hex_path };
    let gen_data = a2l_hex_src.load().expect("gen from A2L+HEX");

    // Write generated data to temp file, read it back as DCM
    let tmp = std::env::temp_dir()
        .join(format!("test_diff_roundtrip_{}.DCM", std::process::id()));
    gen_data.render_to_file(&tmp);

    let dcm_src = CalSource::Dcm(tmp.clone());
    let dcm_data = dcm_src.load().expect("load DCM roundtrip");

    let result = dcm_diff_with_metadata(&gen_data, &dcm_data, &a2l_hex_src, &dcm_src);

    assert_eq!(result.summary.total, 0,
        "Round-trip A2L+HEX -> DCM -> DCM should produce zero diffs, got {}",
        result.summary.total);

    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn test_diff_a2l_vs_a2l_same() {
    let a2l = PathBuf::from("test-dcms/simple_test.a2l");
    let hex = PathBuf::from("test-dcms/simple_test.hex");

    let src1 = CalSource::A2lHex { a2l: a2l.clone(), hex: hex.clone() };
    let src2 = CalSource::A2lHex { a2l: a2l.clone(), hex: hex.clone() };

    let data1 = src1.load().expect("load first A2L+HEX");
    let data2 = src2.load().expect("load second A2L+HEX");

    let result = dcm_diff_with_metadata(&data1, &data2, &src1, &src2);

    assert_eq!(result.summary.total, 0,
        "Same A2L+HEX diffed against itself should produce zero diffs, got {}",
        result.summary.total);
}

#[test]
fn test_diff_a2l_vs_a2l_different() {
    // Gen from simple_test.a2l+hex produces known blocks; diffing against a
    // different DCM file (test1.DCM) should detect differences.
    let a2l_src = CalSource::A2lHex {
        a2l: PathBuf::from("test-dcms/simple_test.a2l"),
        hex: PathBuf::from("test-dcms/simple_test.hex"),
    };
    let dcm_src = CalSource::Dcm(PathBuf::from("test-dcms/test1.DCM"));

    let a2l_data = a2l_src.load().expect("load A2L+HEX");
    let dcm_data = dcm_src.load().expect("load DCM");

    let result = dcm_diff_with_metadata(&a2l_data, &dcm_data, &a2l_src, &dcm_src);

    // These are different calibration sets — there should be differences
    assert!(result.summary.total > 0,
        "A2L+HEX vs different DCM should detect differences");
}

#[test]
fn test_diff_a2l_vs_dcm_different() {
    // Compare A2L+HEX gen result against a known-different DCM file
    let a2l_src = CalSource::A2lHex {
        a2l: PathBuf::from("test-dcms/simple_test.a2l"),
        hex: PathBuf::from("test-dcms/simple_test.hex"),
    };
    let dcm_src = CalSource::Dcm(PathBuf::from("test-dcms/test1_modified.DCM"));

    let a2l_data = a2l_src.load().expect("load A2L+HEX");
    let dcm_data = dcm_src.load().expect("load DCM");

    let result = dcm_diff_with_metadata(&a2l_data, &dcm_data, &a2l_src, &dcm_src);

    assert!(result.summary.total > 0,
        "A2L+HEX vs modified DCM should detect differences");
}

#[test]
fn test_validate_mismatched_counts() {
    // 0 sources
    assert!(validate_and_build_sources(&[], &[], &[]).is_err());
    // 1 source
    assert!(validate_and_build_sources(
        &[PathBuf::from("a.DCM")], &[], &[]
    ).is_err());
    // 3 sources
    assert!(validate_and_build_sources(
        &[PathBuf::from("a.DCM"), PathBuf::from("b.DCM"), PathBuf::from("c.DCM")], &[], &[]
    ).is_err());
    // mismatched a2l/hex
    assert!(validate_and_build_sources(
        &[], &[PathBuf::from("a.a2l")], &[]
    ).is_err());
}
```

- [ ] **Step 2: Run the new integration tests**

Run: `cargo test -- test_diff_dcm_vs_dcm_flags test_diff_a2l_vs_dcm_roundtrip test_diff_a2l_vs_a2l_same test_diff_a2l_vs_a2l_different test_diff_a2l_vs_dcm_different test_validate_mismatched`
Expected: All 6 tests pass.

- [ ] **Step 3: Commit**

```bash
git add tests/test_diff_multi_source.rs
git commit -m "test: add multi-source diff integration tests"
```

---

### Task 7: Final verification

- [ ] **Step 1: Run the full test suite**

Run: `cargo test 2>&1`
Expected: All tests pass, 0 failures.

- [ ] **Step 2: Run clippy**

Run: `cargo clippy 2>&1`
Expected: No warnings.

- [ ] **Step 3: Run rustfmt**

Run: `cargo fmt -- --check`
Expected: No formatting changes needed (or run `cargo fmt` to apply).

- [ ] **Step 4: Verify CLI help output**

Run: `cargo run -- diff --help 2>&1`
Expected: Help text shows `--dcm`, `--a2l`, `-x, --hex`, `-o, --output` flags with no positional arguments.

- [ ] **Step 5: Smoke test with real data**

```bash
cargo run -- diff --dcm test-dcms/test1.DCM --dcm test-dcms/test1_modified.DCM -o output/diff_test.json
```
Expected: Console shows colored diff summary, `output/diff_test.json` is created with valid JSON.

- [ ] **Step 6: Commit any fmt changes**

```bash
git add -A
git commit -m "chore: final formatting and verification" || echo "Nothing to commit"
```
