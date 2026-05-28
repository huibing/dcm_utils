# --approx Diff Flag Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `--approx` CLI flag to the diff command that uses relative tolerance for float comparisons, suppressing diffs from float representation noise.

**Architecture:** Add `approx_eq` methods to `Value` and `Block` alongside existing `PartialEq` impls. Thread an `approx: bool` flag from CLI through the diff functions to select which comparison to use. Fix a pre-existing bug in `Block::PartialEq` for the Map variant that omits axis name comparison.

**Tech Stack:** Rust, `approx` crate (moving from dev-deps to deps), `clap`, `log`

---

### Task 1: Move `approx` crate to dependencies

**Files:**
- Modify: `Cargo.toml:24-26`

- [ ] **Step 1: Move `approx` from dev-dependencies to dependencies**

In `Cargo.toml`, move `approx = "0.5.1"` from the `[dev-dependencies]` section to the `[dependencies]` section (add it after the `ihex = "3.0"` line). Remove it from `[dev-dependencies]`. The `[dev-dependencies]` section should only contain `rstest`.

- [ ] **Step 2: Verify build**

Run: `cargo build`
Expected: Build succeeds with no errors

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: move approx crate from dev-deps to dependencies"
```

---

### Task 2: Add `approx_eq_f64_slice` helper and `Value::approx_eq` method

**Files:**
- Modify: `src/value.rs`

- [ ] **Step 1: Write the failing tests**

Add the following tests to the `#[cfg(test)] mod tests` block in `src/value.rs`:

```rust
use approx::relative_eq;

#[rstest]
fn test_approx_eq_wert_exact() {
    let v1 = Value::WERT(vec![1.0, 2.0, 3.0]);
    let v2 = Value::WERT(vec![1.0, 2.0, 3.0]);
    assert!(v1.approx_eq(&v2));
}

#[rstest]
fn test_approx_eq_wert_near_equal() {
    let v1 = Value::WERT(vec![1.0, 2.0]);
    let v2 = Value::WERT(vec![1.0 + 1e-9, 2.0 + 1e-9]);
    assert!(v1.approx_eq(&v2));
}

#[rstest]
fn test_approx_eq_wert_clearly_different() {
    let v1 = Value::WERT(vec![1.0, 2.0]);
    let v2 = Value::WERT(vec![1.0, 2.1]);
    assert!(!v1.approx_eq(&v2));
}

#[rstest]
fn test_approx_eq_wert_zero_vs_near_zero() {
    let v1 = Value::WERT(vec![0.0]);
    let v2 = Value::WERT(vec![1e-13]);
    assert!(v1.approx_eq(&v2));
}

#[rstest]
fn test_approx_eq_wert_zero_vs_different() {
    let v1 = Value::WERT(vec![0.0]);
    let v2 = Value::WERT(vec![0.01]);
    assert!(!v1.approx_eq(&v2));
}

#[rstest]
fn test_approx_eq_text() {
    let v1 = Value::TEXT(vec!["hello".to_string()]);
    let v2 = Value::TEXT(vec!["hello".to_string()]);
    assert!(v1.approx_eq(&v2));
}

#[rstest]
fn test_approx_eq_text_different() {
    let v1 = Value::TEXT(vec!["hello".to_string()]);
    let v2 = Value::TEXT(vec!["world".to_string()]);
    assert!(!v1.approx_eq(&v2));
}

#[rstest]
fn test_approx_eq_mixed_types() {
    let v1 = Value::WERT(vec![1.0]);
    let v2 = Value::TEXT(vec!["1.0".to_string()]);
    assert!(!v1.approx_eq(&v2));
}

#[rstest]
fn test_approx_eq_different_lengths() {
    let v1 = Value::WERT(vec![1.0, 2.0]);
    let v2 = Value::WERT(vec![1.0]);
    assert!(!v1.approx_eq(&v2));
}

#[rstest]
fn test_approx_eq_empty_wert() {
    let v1 = Value::WERT(vec![]);
    let v2 = Value::WERT(vec![]);
    assert!(v1.approx_eq(&v2));
}

#[rstest]
fn test_approx_eq_f64_slice() {
    let a = vec![1.0, 2.0, 3.0];
    let b = vec![1.0 + 1e-9, 2.0 + 1e-9, 3.0 + 1e-9];
    assert!(approx_eq_f64_slice(&a, &b));
}

#[rstest]
fn test_approx_eq_f64_slice_zero() {
    let a = vec![0.0];
    let b = vec![1e-13];
    assert!(approx_eq_f64_slice(&a, &b));
}

#[rstest]
fn test_approx_eq_f64_slice_different() {
    let a = vec![1.0, 2.0];
    let b = vec![1.0, 2.1];
    assert!(!approx_eq_f64_slice(&a, &b));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib value::tests`
Expected: Compile errors — `approx_eq` and `approx_eq_f64_slice` don't exist yet

- [ ] **Step 3: Implement `approx_eq_f64_slice` and `Value::approx_eq`**

Add the following to `src/value.rs` (inside the `impl Value` block, after the `try_into_f64` method):

```rust
pub fn approx_eq(&self, other: &Self) -> bool {
    match (self, other) {
        (Value::WERT(v1), Value::WERT(v2)) => approx_eq_f64_slice(v1, v2),
        (Value::TEXT(v1), Value::TEXT(v2)) => v1 == v2,
        _ => false,
    }
}
```

Add the following as a free function in `src/value.rs` (outside the impl block, before the `impl From<ValueAttr>` block). Also add `use approx::relative_eq;` at the top of the file (module level, needed for production code):

```rust
use approx::relative_eq;
```

```rust
pub(crate) fn approx_eq_f64_slice(a: &[f64], b: &[f64]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter()
        .zip(b.iter())
        .all(|(x, y)| relative_eq!(x, y, max_relative = 1e-8, epsilon = 1e-12))
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib value::tests`
Expected: All tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/value.rs
git commit -m "feat: add approx_eq method to Value and approx_eq_f64_slice helper"
```

---

### Task 3: Fix `Block::PartialEq` bug and add `Block::approx_eq`

**Files:**
- Modify: `src/block.rs:82-97`

- [ ] **Step 1: Write the failing test for the PartialEq bug**

Add the following test to the `#[cfg(test)] mod tests` block in `src/block.rs`:

```rust
use crate::blocks::GRUPPENKENNFELD;

#[rstest]
fn test_block_partial_eq_map_checks_axis_names() {
    let map1 = GRUPPENKENNFELD::from_f64(
        "test_map", vec![vec![1.0, 2.0], vec![3.0, 4.0]],
        vec![1.0, 2.0], vec![0.0, 1.0],
        "axis_x_v1", "axis_y_v1",
        "desc", "unit_w", "unit_x", "unit_y",
    );
    let map2 = GRUPPENKENNFELD::from_f64(
        "test_map", vec![vec![1.0, 2.0], vec![3.0, 4.0]],
        vec![1.0, 2.0], vec![0.0, 1.0],
        "axis_x_v1", "axis_y_v2", // different y_axis_name
        "desc", "unit_w", "unit_x", "unit_y",
    );
    let b1 = Block::Map(map1);
    let b2 = Block::Map(map2);
    assert_ne!(b1, b2, "Block::PartialEq should detect different y_axis_name");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib block::tests::test_block_partial_eq_map_checks_axis_names`
Expected: FAIL — `Block::PartialEq` currently omits axis name comparison

- [ ] **Step 3: Fix `Block::PartialEq` to check axis names for Map variant**

In `src/block.rs`, replace the Map arm of `PartialEq`:

```rust
(Block::Map(m1), Block::Map(m2)) => {
    m1.value_flat == m2.value_flat
        && m1.x_axis == m2.x_axis
        && m1.y_axis == m2.y_axis
        && m1.x_axis_name == m2.x_axis_name
        && m1.y_axis_name == m2.y_axis_name
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test --lib block::tests::test_block_partial_eq_map_checks_axis_names`
Expected: PASS

- [ ] **Step 5: Add `Block::approx_eq` method**

Add the following method to the `impl Block` block in `src/block.rs`:

```rust
pub fn approx_eq(&self, other: &Self) -> bool {
    match (self, other) {
        (Block::Constant(c1), Block::Constant(c2)) => c1.value.approx_eq(&c2.value),
        (Block::ConstantBlock(b1), Block::ConstantBlock(b2)) => b1.value.approx_eq(&b2.value),
        (Block::Table(t1), Block::Table(t2)) => {
            t1.value.approx_eq(&t2.value)
                && approx_eq_f64_slice(&t1.axis, &t2.axis)
                && t1.axis_var_name == t2.axis_var_name
        }
        (Block::Distribution(d1), Block::Distribution(d2)) => d1.value.approx_eq(&d2.value),
        (Block::Map(m1), Block::Map(m2)) => {
            m1.value_flat.approx_eq(&m2.value_flat)
                && approx_eq_f64_slice(&m1.x_axis, &m2.x_axis)
                && approx_eq_f64_slice(&m1.y_axis, &m2.y_axis)
                && m1.x_axis_name == m2.x_axis_name
                && m1.y_axis_name == m2.y_axis_name
        }
        _ => false,
    }
}
```

Also add the import at the top of `src/block.rs`:

```rust
use crate::value::approx_eq_f64_slice;
```

- [ ] **Step 6: Write tests for `Block::approx_eq`**

Add to the tests module in `src/block.rs`:

```rust
#[rstest]
fn test_block_approx_eq_constant() {
    let c1 = FESTWERT::from_f64("test".to_string(), 1.0, "desc".to_string(), "unit".to_string());
    let c2 = FESTWERT::from_f64("test".to_string(), 1.0 + 1e-9, "desc".to_string(), "unit".to_string());
    let b1 = Block::Constant(c1);
    let b2 = Block::Constant(c2);
    assert!(b1.approx_eq(&b2));
    assert_ne!(b1, b2);
}

#[rstest]
fn test_block_approx_eq_constant_block() {
    let cb1 = FESTWERTEBLOCK::from_f64("test_cb".to_string(), vec![1.0, 2.0, 3.0], "desc".to_string(), "unit".to_string());
    let cb2 = FESTWERTEBLOCK::from_f64("test_cb".to_string(), vec![1.0 + 1e-9, 2.0, 3.0], "desc".to_string(), "unit".to_string());
    let b1 = Block::ConstantBlock(cb1);
    let b2 = Block::ConstantBlock(cb2);
    assert!(b1.approx_eq(&b2));
    assert_ne!(b1, b2);
}

#[rstest]
fn test_block_approx_eq_table() {
    let t1 = GRUPPENKENNLINIE::from_f64(
        "test_tbl", &[1.0, 2.0, 3.0],
        "desc", "unit_w", "unit_x",
        "axis_x", &[0.0, 10.0, 20.0],
    );
    let t2 = GRUPPENKENNLINIE::from_f64(
        "test_tbl", &[1.0 + 1e-9, 2.0, 3.0],
        "desc", "unit_w", "unit_x",
        "axis_x", &[0.0, 10.0 + 1e-9, 20.0],
    );
    let b1 = Block::Table(t1);
    let b2 = Block::Table(t2);
    assert!(b1.approx_eq(&b2));
    assert_ne!(b1, b2);
}

#[rstest]
fn test_block_approx_eq_distribution() {
    let d1 = STUETZSTELLENVERTEILUNG::from_f64("test_dist", "desc", &[0.0, 10.0, 20.0], "unit");
    let d2 = STUETZSTELLENVERTEILUNG::from_f64("test_dist", "desc", &[0.0, 10.0 + 1e-9, 20.0], "unit");
    let b1 = Block::Distribution(d1);
    let b2 = Block::Distribution(d2);
    assert!(b1.approx_eq(&b2));
    assert_ne!(b1, b2);
}

#[rstest]
fn test_block_approx_eq_map() {
    let map1 = GRUPPENKENNFELD::from_f64(
        "test_map", vec![vec![1.0, 2.0], vec![3.0, 4.0]],
        vec![1.0, 2.0], vec![0.0, 1.0],
        "axis_x", "axis_y",
        "desc", "unit_w", "unit_x", "unit_y",
    );
    let map2 = GRUPPENKENNFELD::from_f64(
        "test_map", vec![vec![1.0 + 1e-9, 2.0], vec![3.0, 4.0]],
        vec![1.0, 2.0], vec![0.0, 1.0],
        "axis_x", "axis_y",
        "desc", "unit_w", "unit_x", "unit_y",
    );
    let b1 = Block::Map(map1);
    let b2 = Block::Map(map2);
    assert!(b1.approx_eq(&b2));
    assert_ne!(b1, b2);
}
```

Note: These tests use `from_f64` constructors on each block type. Verify these constructors exist and have the right signatures by checking the individual block files in `src/blocks/`. If `STUETZSTELLENVERTEILUNG::from_f64` or `GRUPPENKENNLINIE::from_f64` don't exist, they will need to be added (following the pattern of `FESTWERT::from_f64` and `GRUPPENKENNFELD::from_f64`).

- [ ] **Step 7: Run all block tests**

Run: `cargo test --lib block::tests`
Expected: All tests PASS

- [ ] **Step 8: Commit**

```bash
git add src/block.rs
git commit -m "fix: add axis_name check to Block::PartialEq for Map; add Block::approx_eq"
```

---

### Task 4: Thread `approx` flag through diff functions

**Files:**
- Modify: `src/diff.rs:258-332`
- Modify: `src/lib.rs:8-10`

- [ ] **Step 1: Write failing test for approx diff**

Add to the `#[cfg(test)] mod tests` block in `src/diff.rs`:

```rust
use crate::block::Block;
use crate::blocks::FESTWERT;
use crate::DcmData;
use indexmap::IndexMap;

fn make_dcm_with_constant(name: &str, value: f64) -> DcmData {
    let festwert = FESTWERT::from_f64(name.to_string(), value, "desc".to_string(), "unit".to_string());
    let mut blocks = IndexMap::new();
    blocks.insert(name.to_string(), Block::Constant(festwert));
    DcmData { blocks }
}

#[test]
fn test_dcm_diff_approx_suppresses_float_noise() {
    let left = make_dcm_with_constant("param1", 1.0);
    let right = make_dcm_with_constant("param1", 1.0 + 1e-9);

    // Exact comparison: should report a change
    let exact_diff = dcm_diff(&left, &right);
    assert_eq!(exact_diff.len(), 1);

    // Approximate comparison: should suppress the noise diff
    let approx_diff = dcm_diff_with_metadata(
        &left,
        &right,
        &CalSource::Dcm(PathBuf::from("left.DCM")),
        &CalSource::Dcm(PathBuf::from("right.DCM")),
    );
    // dcm_diff_with_metadata currently doesn't accept approx — test will fail until we add it
    assert_eq!(approx_diff.differences.len(), 0);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib diff::tests::test_dcm_diff_approx_suppresses_float_noise`
Expected: FAIL — `dcm_diff_with_metadata` doesn't accept `approx` yet, so approx diff still reports 1 change

- [ ] **Step 3: Add `approx` parameter to diff functions**

In `src/diff.rs`, make these changes:

1. Change `dcm_diff` to pass `false`:
```rust
pub fn dcm_diff(left: &DcmData, right: &DcmData) -> Vec<DcmDiff> {
    dcm_diff_with_details(left, right, false, false)
}
```

2. Add `approx` parameter to `dcm_diff_with_details`:
```rust
fn dcm_diff_with_details(left: &DcmData, right: &DcmData, _detailed: bool, approx: bool) -> Vec<DcmDiff> {
```

3. In `dcm_diff_with_details`, change the comparison at line 290:
```rust
let blocks_equal = if approx {
    left_block.approx_eq(right_block)
} else {
    left_block == right_block
};
if !blocks_equal {
```

4. Add `approx` parameter to `dcm_diff_with_metadata`:
```rust
pub fn dcm_diff_with_metadata(
    left: &DcmData,
    right: &DcmData,
    left_src: &CalSource,
    right_src: &CalSource,
    approx: bool,
) -> DcmDiffResult {
    let metadata = DiffMetadata::new(&left_src.label(), &right_src.label());
    let differences = dcm_diff_with_details(left, right, true, approx);
    DcmDiffResult::new(metadata, differences)
}
```

5. Add `use crate::block::Block;` import if not already present (it is already imported).

6. Add `use log::warn;` to the imports at the top of `src/diff.rs` (change `use log::info;` to `use log::{info, warn};`).

7. Add the warning log at the beginning of `dcm_diff_with_details` when `approx` is true:
```rust
if approx {
    warn!("Approximate comparison enabled: float differences within relative tolerance 1e-8 (epsilon 1e-12) will be treated as equal");
}
```

- [ ] **Step 4: Update `generate_change_description` to accept `approx`**

Change the signature to:
```rust
fn generate_change_description(name: &str, left: &Block, right: &Block, approx: bool) -> String {
```

Update each internal value/axis comparison to use approximate comparison when `approx` is true. For example, the Table arm changes from:
```rust
if left_table.value != right_table.value {
```
to:
```rust
let values_equal = if approx { left_table.value.approx_eq(&right_table.value) } else { left_table.value == right_table.value };
if !values_equal {
```

Apply the same pattern to all value comparisons within `generate_change_description`. Only the existing Value comparisons need the approx treatment — the function currently does not compare axis breakpoint values directly (only axis lengths and axis names). Specifically:
- Table: `left_table.value` vs `right_table.value`
- Map: `left_map.value_flat` vs `right_map.value_flat`
- ConstantBlock: `left_cb.value` vs `right_cb.value`
- Constant: `left_c.value` vs `right_c.value`
- Distribution: `left_d.value` vs `right_d.value`

Length, dimension, and string comparisons remain exact (`==`) regardless of `approx`.

Update the call site in `dcm_diff_with_details`:
```rust
let description = generate_change_description(name, left_block, right_block, approx);
```

- [ ] **Step 5: Update the public API in `lib.rs`**

In `src/lib.rs`, the re-export of `dcm_diff_with_metadata` already exists. No change needed to the `pub use` line since the signature change is in the same crate. But verify the re-export compiles.

- [ ] **Step 6: Update the failing test to pass `approx: true`**

Update the test in `src/diff.rs`:
```rust
#[test]
fn test_dcm_diff_approx_suppresses_float_noise() {
    let left = make_dcm_with_constant("param1", 1.0);
    let right = make_dcm_with_constant("param1", 1.0 + 1e-9);

    // Exact comparison: should report a change
    let exact_diff = dcm_diff(&left, &right);
    assert_eq!(exact_diff.len(), 1);

    // Approximate comparison: should suppress the noise diff
    let approx_result = dcm_diff_with_metadata(
        &left,
        &right,
        &CalSource::Dcm(PathBuf::from("left.DCM")),
        &CalSource::Dcm(PathBuf::from("right.DCM")),
        true,
    );
    assert_eq!(approx_result.differences.len(), 0);
}
```

- [ ] **Step 7: Add integration test for approx diff with multiple block types**

Add to the `#[cfg(test)] mod tests` block in `src/diff.rs`:

```rust
#[test]
fn test_dcm_diff_approx_suppresses_noise_multiple_types() {
    let left = make_dcm_with_constant("param1", 1.0);
    let mut right_blocks = IndexMap::new();
    let c = FESTWERT::from_f64("param1".to_string(), 1.0 + 1e-9, "desc".to_string(), "unit".to_string());
    right_blocks.insert("param1".to_string(), Block::Constant(c));
    let right = DcmData { blocks: right_blocks };

    // Exact: 1 change
    let exact = dcm_diff(&left, &right);
    assert_eq!(exact.len(), 1);

    // Approx: 0 changes
    let approx_result = dcm_diff_with_metadata(
        &left, &right,
        &CalSource::Dcm(PathBuf::from("left")),
        &CalSource::Dcm(PathBuf::from("right")),
        true,
    );
    assert_eq!(approx_result.differences.len(), 0);
}
```

- [ ] **Step 8: Run all diff tests**

Run: `cargo test --lib diff::tests`
Expected: All tests PASS

- [ ] **Step 9: Commit**

```bash
git add src/diff.rs src/lib.rs
git commit -m "feat: thread approx flag through diff functions with approximate comparison"
```

---

### Task 5: Add `--approx` CLI flag and wire it up

**Files:**
- Modify: `src/main.rs:111-124, 207-308`

- [ ] **Step 1: Add `--approx` flag to the Diff subcommand**

In `src/main.rs`, add the flag to the `Diff` variant after the `output` field:

```rust
/// Use approximate comparison for floating-point values (relative tolerance 1e-8)
#[arg(long, default_value_t = false)]
approx: bool,
```

- [ ] **Step 2: Pass `approx` to `dcm_diff_with_metadata` in the Diff match arm**

In the `Commands::Diff` match arm, update the call:

```rust
let result = dcm_diff_with_metadata(&left_data, &right_data, &left_src, &right_src, approx);
```

Also destructure `approx` from the Diff variant:

```rust
Commands::Diff {
    dcm,
    a2l,
    hex,
    output,
    approx,
} => {
```

- [ ] **Step 3: Verify build and test**

Run: `cargo build && cargo test`
Expected: Build succeeds, all tests pass

- [ ] **Step 4: Test the CLI flag manually**

Run: `cargo run -- diff --dcm test-dcms/test_sample_673.DCM --dcm test-dcms/test_sample_673.DCM`
Expected: No differences reported (exact same file)

Run: `cargo run -- diff --dcm test-dcms/test_sample_673.DCM --dcm test-dcms/test_sample_673.DCM --approx`
Expected: Same result, plus a WARN log about approximate comparison

Run: `cargo run -- --help` and `cargo run -- diff --help`
Expected: `--approx` flag appears in the diff subcommand help

- [ ] **Step 5: Commit**

```bash
git add src/main.rs
git commit -m "feat: add --approx CLI flag to diff command"
```

---

### Task 6: Run full test suite and final checks

**Files:**
- None (verification only)

- [ ] **Step 1: Run full test suite**

Run: `cargo test`
Expected: All tests pass

- [ ] **Step 2: Run clippy**

Run: `cargo clippy`
Expected: No warnings or errors

- [ ] **Step 3: Run fmt**

Run: `cargo fmt --check`
Expected: No formatting issues. If there are, run `cargo fmt` and commit.

- [ ] **Step 4: Final commit if fmt was needed**

```bash
git add -A
git commit -m "style: apply cargo fmt"
```
