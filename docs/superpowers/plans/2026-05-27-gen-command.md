# GEN Command Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `gen` CLI subcommand that extracts calibration values from A2L+HEX files and writes them to a DCM file using the a2ldeser library.

**Architecture:** Single `src/gen.rs` module encapsulates all A2L/HEX logic and exposes `gen_dcm_data(a2l: &Path, hex: &Path) -> Result<DcmData, Box<dyn Error>>`. Main.rs adds a `Gen` CLI variant that calls this function and writes the output. The a2ldeser library is added as a git dependency; `a2lfile` is added as an explicit direct dependency (needed for `a2lfile::load` within gen.rs).

**Tech Stack:** Rust 2021, a2ldeser (git), a2lfile v3.3, clap derive, indexmap, handlebars

**Spec:** `docs/superpowers/specs/2026-05-27-gen-command-design.md`

---

## File Structure

| Action | Path | Purpose |
|--------|------|---------|
| Create | `src/gen.rs` | A2L+HEX → DCM conversion logic |
| Modify | `Cargo.toml` | Add a2ldeser + a2lfile dependencies |
| Modify | `src/lib.rs:1-6` | Add `pub mod gen;` |
| Modify | `src/main.rs:1-19,25-120` | Add `Gen` variant and import |

---

### Task 1: Add dependencies to Cargo.toml

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Add a2ldeser and a2lfile dependencies**

Add two lines to the `[dependencies]` section:

```toml
a2ldeser = { git = "https://github.com/huibing/a2ldeser" }
a2lfile = "3.3"
```

Why `a2lfile` as explicit dependency: Rust 2021 edition makes all dependencies private; transitive deps are not accessible. `gen.rs` needs `a2lfile::load()` to parse the A2L file before passing it to the a2ldeser `Extractor`.

- [ ] **Step 2: Run cargo fetch to verify dependency resolution**

```bash
cargo fetch
```

Expected: Dependencies resolve without errors.

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "feat: add a2ldeser and a2lfile dependencies for GEN command"
```

---

### Task 2: Create gen.rs conversion module

**Files:**
- Create: `src/gen.rs`

This module contains all A2L/HEX-to-DCM conversion logic. It exposes a single public function `gen_dcm_data()` and keeps internal helpers private.

- [ ] **Step 1: Write the module skeleton with imports**

```rust
use std::collections::HashMap;
use std::error::Error;
use std::ffi::OsString;
use std::path::Path;

use a2ldeser::extractor::{ExtractedObject, Extractor, PhysicalValue};
use a2ldeser::hex_reader::HexMemory;

use crate::DcmData;
use crate::block::Block;
use crate::blocks::{FESTWERT, FESTWERTEBLOCK, GRUPPENKENNLINIE, GRUPPENKENNFELD, STUETZSTELLENVERTEILUNG};
use indexmap::IndexMap;
```

- [ ] **Step 2: Write `gen_dcm_data()` public entry point**

```rust
/// Parse A2L+HEX, extract all calibration characteristics, and return DcmData
pub fn gen_dcm_data(a2l: &Path, hex: &Path) -> Result<DcmData, Box<dyn Error>> {
    // 1. Load A2L file
    let a2l_path = OsString::from(a2l.as_os_str());
    let (a2l_obj, _warnings) = a2lfile::load(a2l_path, None, false)
        .map_err(|e| format!("Failed to parse A2L file '{}': {}", a2l.display(), e))?;
    let module = &a2l_obj.project.module[0];

    // 2. Load HEX file
    let hex_mem = HexMemory::from_file(hex)
        .map_err(|e| format!("Failed to parse HEX file '{}': {}", hex.display(), e))?;

    // 3. Extract all characteristics
    let extractor = Extractor::new(module, &hex_mem);
    let report = extractor.extract_all();

    // 4. Print failure summary to stderr
    for fail in &report.failures {
        eprintln!("SKIP: {} - {:?}", fail.name, fail.error);
    }
    eprintln!(
        "Extracted {}/{} characteristics",
        report.successes.len(),
        report.total()
    );

    // 5. Convert ExtractedObject list → IndexMap<String, Block>
    let blocks = extracted_to_dcm_blocks(report.successes, module);

    Ok(DcmData::from_blocks(blocks))
}
```

- [ ] **Step 3: Write `extracted_to_dcm_blocks()` — the core converter**

This function:
1. Builds a `long_identifier` lookup map from the A2L module
2. Groups ExtractedObjects by type
3. Inserts blocks in the specified order: FESTWERT → FESTWERTEBLOCK → STUETZSTELLENVERTEILUNG → GRUPPENKENNLINIE → GRUPPENKENNFELD

```rust
fn extracted_to_dcm_blocks(
    objects: Vec<ExtractedObject>,
    module: &a2lfile::Module,
) -> IndexMap<String, Block> {
    let mut blocks = IndexMap::new();

    // Build long_identifier lookup map from A2L characteristics
    let mut langname_map: HashMap<String, String> = HashMap::new();
    for chr in &module.characteristic {
        langname_map.insert(chr.name.clone(), chr.long_identifier.clone());
    }
    // Also index AXIS_PTS by name for standalone axis metadata
    for apt in &module.axis_pts {
        langname_map.entry(apt.name.clone())
            .or_insert_with(|| apt.long_identifier.clone());
    }

    // Helper: get langname, fall back to the name itself
    let get_langname = |name: &str| -> String {
        langname_map.get(name).cloned().unwrap_or_else(|| name.to_string())
    };

    // Partition objects by type
    let mut values = Vec::new();
    let mut val_blks = Vec::new();
    let mut asciis = Vec::new();
    let mut curves = Vec::new();
    let mut maps = Vec::new();
    let mut axis_pts_list = Vec::new();

    for obj in objects {
        match obj {
            ExtractedObject::Value(v) => values.push(v),
            ExtractedObject::ValBlk(v) => val_blks.push(v),
            ExtractedObject::Ascii(a) => asciis.push(a),
            ExtractedObject::Curve(c) => curves.push(c),
            ExtractedObject::Map(m) => maps.push(m),
            ExtractedObject::AxisPts(a) => axis_pts_list.push(a),
        }
    }

    // Group 1: FESTWERT (Value)
    for v in values {
        let langname = get_langname(&v.name);
        let block = extracted_value_to_block(&v, &langname);
        blocks.insert(v.name.clone(), block);
    }

    // Group 2: FESTWERT (Ascii)
    for a in asciis {
        let langname = get_langname(&a.name);
        let block = Block::Constant(FESTWERT::from_string(
            a.name.clone(), a.text, langname, String::new(),
        ));
        blocks.insert(a.name.clone(), block);
    }

    // Group 3: FESTWERTEBLOCK
    for vb in val_blks {
        let langname = get_langname(&vb.name);
        let block = extracted_valblk_to_block(&vb, &langname);
        blocks.insert(vb.name.clone(), block);
    }

    // Group 4: STUETZSTELLENVERTEILUNG (standalone + derived)
    // Standalone AxisPts first (they take priority in collisions)
    for apt in &axis_pts_list {
        let langname = get_langname(&apt.name);
        let block = Block::Distribution(STUETZSTELLENVERTEILUNG::from_f64(
            &apt.name, &langname, &apt.values, &apt.unit,
        ));
        blocks.insert(apt.name.clone(), block);
    }
    // Derived axes from Curves
    for c in &curves {
        let axis_name = format!("{}_X", c.name);
        if !blocks.contains_key(&axis_name) {
            let langname = get_langname(&c.name);
            let block = Block::Distribution(STUETZSTELLENVERTEILUNG::from_f64(
                &axis_name, &langname, &c.x_axis, &c.x_unit,
            ));
            blocks.insert(axis_name, block);
        } else {
            eprintln!("WARN: Derived axis '{}' collides with existing AXIS_PTS, skipped", axis_name);
        }
    }
    // Derived axes from Maps
    for m in &maps {
        let x_axis_name = format!("{}_X", m.name);
        let y_axis_name = format!("{}_Y", m.name);
        for (axis_name, axis_values, axis_unit) in [
            (&x_axis_name, &m.x_axis, &m.x_unit),
            (&y_axis_name, &m.y_axis, &m.y_unit),
        ] {
            if !blocks.contains_key(axis_name.as_str()) {
                let langname = get_langname(&m.name);
                let block = Block::Distribution(STUETZSTELLENVERTEILUNG::from_f64(
                    axis_name, &langname, axis_values, axis_unit,
                ));
                blocks.insert(axis_name.clone(), block);
            } else {
                eprintln!("WARN: Derived axis '{}' collides with existing AXIS_PTS, skipped", axis_name);
            }
        }
    }

    // Group 5: GRUPPENKENNLINIE (skip if contains verbal values)
    for c in curves {
        let langname = get_langname(&c.name);
        if let Some(block) = extracted_curve_to_block(&c, &langname) {
            blocks.insert(c.name.clone(), block);
        }
    }

    // Group 6: GRUPPENKENNFELD (skip if contains verbal values)
    for m in maps {
        let langname = get_langname(&m.name);
        if let Some(block) = extracted_map_to_block(&m, &langname) {
            blocks.insert(m.name.clone(), block);
        }
    }

    blocks
}
```

- [ ] **Step 4: Write `extracted_value_to_block()`**

```rust
fn extracted_value_to_block(v: &a2ldeser::extractor::ExtractedValue, langname: &str) -> Block {
    let unit = v.unit.clone().unwrap_or_default();
    match &v.physical {
        PhysicalValue::Numeric(n) => {
            Block::Constant(FESTWERT::from_f64(v.name.clone(), *n, langname.to_string(), unit))
        }
        PhysicalValue::Verbal(s) => {
            Block::Constant(FESTWERT::from_string(v.name.clone(), s.clone(), langname.to_string(), unit))
        }
    }
}
```

- [ ] **Step 5: Write `extracted_valblk_to_block()`**

If any element is Verbal, convert entire block to TEXT. Otherwise use WERT.

```rust
fn extracted_valblk_to_block(vb: &a2ldeser::extractor::ExtractedValBlk, langname: &str) -> Block {
    let unit = vb.unit.clone().unwrap_or_default();
    let has_verbal = vb.values.iter().any(|pv| matches!(pv, PhysicalValue::Verbal(_)));

    if has_verbal {
        let strings: Vec<String> = vb.values.iter().map(|pv| match pv {
            PhysicalValue::Numeric(n) => format!("{}", n),
            PhysicalValue::Verbal(s) => s.clone(),
        }).collect();
        Block::ConstantBlock(FESTWERTEBLOCK::from_string(
            vb.name.clone(), strings, langname.to_string(), unit,
        ))
    } else {
        let nums: Vec<f64> = vb.values.iter().map(|pv| match pv {
            PhysicalValue::Numeric(n) => *n,
            _ => unreachable!(),
        }).collect();
        Block::ConstantBlock(FESTWERTEBLOCK::from_f64(
            vb.name.clone(), nums, langname.to_string(), unit,
        ))
    }
}
```

- [ ] **Step 6: Write `extracted_curve_to_block()`**

CURVE with any Verbal value is skipped entirely with a warning. Returns `Option<Block>`.

```rust
fn extracted_curve_to_block(c: &a2ldeser::extractor::ExtractedCurve, langname: &str) -> Option<Block> {
    let axis_name = format!("{}_X", c.name);
    let unit = c.unit.clone().unwrap_or_default();
    let unit_x = c.x_unit.clone().unwrap_or_default();

    let has_verbal = c.values.iter().any(|pv| matches!(pv, PhysicalValue::Verbal(_)));
    if has_verbal {
        eprintln!("WARN: Skipping CURVE '{}' — contains verbal values, not representable in DCM", c.name);
        return None;
    }

    let nums: Vec<f64> = c.values.iter().map(|pv| match pv {
        PhysicalValue::Numeric(n) => *n,
        _ => unreachable!(),
    }).collect();

    Some(Block::Table(GRUPPENKENNLINIE::from_f64(
        &c.name,
        &nums,
        langname,
        &unit,
        &unit_x,
        &axis_name,
        &c.x_axis,
    )))
}
```

- [ ] **Step 7: Write `extracted_map_to_block()`**

MAP with any Verbal value is skipped entirely.

```rust
fn extracted_map_to_block(m: &a2ldeser::extractor::ExtractedMap, langname: &str) -> Option<Block> {
    let x_axis_name = format!("{}_X", m.name);
    let y_axis_name = format!("{}_Y", m.name);
    let unit_w = m.unit.clone().unwrap_or_default();
    let unit_x = m.x_unit.clone().unwrap_or_default();
    let unit_y = m.y_unit.clone().unwrap_or_default();

    let has_verbal = m.values.iter().any(|row| {
        row.iter().any(|pv| matches!(pv, PhysicalValue::Verbal(_)))
    });
    if has_verbal {
        eprintln!("WARN: Skipping MAP '{}' — contains verbal values, not representable in DCM", m.name);
        return None;
    }

    let values_2d: Vec<Vec<f64>> = m.values.iter().map(|row| {
        row.iter().map(|pv| match pv {
            PhysicalValue::Numeric(n) => *n,
            _ => unreachable!(),
        }).collect()
    }).collect();

    Some(Block::Map(GRUPPENKENNFELD::from_f64(
        &m.name,
        values_2d,
        m.x_axis.clone(),
        m.y_axis.clone(),
        &x_axis_name,
        &y_axis_name,
        langname,
        &unit_w,
        &unit_x,
        &unit_y,
    )))
}
```

- [ ] **Step 8: Commit**

```bash
git add src/gen.rs
git commit -m "feat(gen): add A2L+HEX to DCM conversion module"
```

---

### Task 3: Register gen module in lib.rs

**Files:**
- Modify: `src/lib.rs:1-6`

- [ ] **Step 1: Add `pub mod gen;` to lib.rs**

Add `pub mod gen;` after the existing module declarations. Updated module list:

```rust
pub mod attr;
pub mod block;
pub mod blocks;
pub mod value;
pub mod diff;
pub mod gen;
```

- [ ] **Step 2: Verify it compiles**

```bash
cargo build
```

Expected: `src/gen.rs` compiles without errors. (May have unused import warnings initially.)

- [ ] **Step 3: Commit**

```bash
git add src/lib.rs
git commit -m "feat: register gen module in lib.rs"
```

---

### Task 4: Add Gen CLI subcommand

**Files:**
- Modify: `src/main.rs:1-19` (imports)
- Modify: `src/main.rs:25-120` (Commands enum)

- [ ] **Step 1: Add import for gen module**

Add to the imports at the top of `main.rs`:

```rust
use dcm_utils::gen;
```

- [ ] **Step 2: Add Gen variant to Commands enum**

Insert after the `Diff` variant, before the closing `}` of the enum:

```rust
/// Generate DCM file from A2L and HEX calibration files
///
/// Extracts all calibration characteristics from an A2L file and HEX flash image,
/// converting them to DCM format. Failed extractions are skipped with warnings.
///
/// ## Examples
///
/// Basic usage:
///
///     dcm_utils gen --a2l calibration.a2l --hex flash.hex
///
/// Custom output file:
///
///     dcm_utils gen --a2l calibration.a2l --hex flash.hex --output all_cali.DCM
Gen {
    /// Path to the A2L calibration description file
    #[arg(short, long)]
    a2l: PathBuf,
    /// Path to the Intel HEX flash image
    #[arg(short, long)]
    hex: PathBuf,
    /// Output DCM file path
    #[arg(short, long, default_value = "generated.dcm")]
    output: PathBuf,
},
```

- [ ] **Step 3: Add match arm for Gen command**

Insert after the `Commands::Diff` arm, before the closing `}` of the match:

```rust
Commands::Gen { a2l, hex, output } => {
    let dcm_data = gen::gen_dcm_data(&a2l, &hex)
        .expect("Failed to generate DCM data");
    dcm_data.render_to_file(&output);
    println!("DCM file written to: {}", output.display().to_string().blue());
},
```

- [ ] **Step 4: Run cargo build to verify**

```bash
cargo build
```

Expected: Compiles cleanly, new `gen` subcommand appears in `--help`.

- [ ] **Step 5: Commit**

```bash
git add src/main.rs
git commit -m "feat: add gen CLI subcommand for A2L+HEX to DCM conversion"
```

---

### Task 5: Tests — integration and edge cases

**Files:**
- Create: `test-dcms/simple_test.a2l` (fixture: VALUE + CURVE + VAL_BLK)
- Create: `test-dcms/simple_test.hex` (matching HEX fixture)
- Modify: `src/gen.rs` (unit tests for conversion helpers)
- Modify: `src/main.rs` (integration test)

- [ ] **Step 1: Create A2L test fixture with VALUE, CURVE, and VAL_BLK**

Create `test-dcms/simple_test.a2l`:

```
ASAP2_VERSION 1 61
/begin PROJECT test_project "Test Project"
/begin MODULE test_module "Test Module"

/begin MOD_COMMON "Common"
/end MOD_COMMON

/begin IF_DATA XCP
/end IF_DATA

/begin MOD_PAR "Parameters"
/end MOD_PAR

/begin RECORD_LAYOUT Scalar_FLOAT64
  FNC_VALUES 1 FLOAT64_IEEE COLUMN_DIR
/end RECORD_LAYOUT

/begin RECORD_LAYOUT Curve_3x_FLOAT64
  FNC_VALUES 3 FLOAT64_IEEE COLUMN_DIR
/end RECORD_LAYOUT

/begin RECORD_LAYOUT ValBlk_4x_UBYTE
  FNC_VALUES 4 UBYTE COLUMN_DIR
/end RECORD_LAYOUT

/begin CHARACTERISTIC
  test_scalar
  "Test scalar value"
  VALUE
  0x8000
  Scalar_FLOAT64
  0
  cm_identical
  100.0 -100.0
/end CHARACTERISTIC

/begin CHARACTERISTIC
  test_curve
  "Test curve"
  CURVE
  0x8030
  Curve_3x_FLOAT64
  0
  cm_linear
  6000.0 -50.0
  FIX_AXIS_PAR 3 0.0 10.0
  FLOAT64_IEEE
/end CHARACTERISTIC

/begin CHARACTERISTIC
  test_valblk
  "Test value block"
  VAL_BLK
  0x8050
  ValBlk_4x_UBYTE
  0
  cm_linear
  100.0 0.0
  /begin AXIS_DESCR
    STD_AXIS
    FLOAT64_IEEE
    cm_identical
    4 0 4
  /end AXIS_DESCR
/end CHARACTERISTIC

/begin COMPU_METHOD cm_identical
  "identical"
  IDENTICAL "%6.2" "unitless"
/end COMPU_METHOD

/begin COMPU_METHOD cm_linear
  "linear"
  LINEAR "%6.2" "deg"
/end COMPU_METHOD

/end MODULE
/end PROJECT
```

- [ ] **Step 2: Create matching HEX test fixture**

Create `test-dcms/simple_test.hex`. This covers all three characteristics:

```
:020000040000FA
:088000000000000000004540B1       ; test_scalar at 0x8000: 42.0 (FLOAT64_IEEE)
:1880300000000000000000000000000000F03F0000000000000840B0  ; test_curve at 0x8030: 1.0, 2.0, 3.0
:048050000102030400              ; test_valblk at 0x8050: 1, 2, 3, 4 (UBYTE)
:00000001FF
```

- [ ] **Step 3: Write unit tests in gen.rs for conversion helpers**

Add a `#[cfg(test)]` module at the bottom of `src/gen.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use a2ldeser::extractor::{ExtractedValue, ExtractedValBlk, ExtractedCurve};
    use a2ldeser::extractor::PhysicalValue;

    #[test]
    fn test_extracted_value_numeric_to_festwert() {
        // NOTE: The `raw` field type is `A2lValue` from a2ldeser::types.
        // Use `A2lValue::F64(42.0)` or check the actual type in the a2ldeser API.
        // This field is not inspected by `extracted_value_to_block()`, so any
        // valid A2lValue will work for testing.
        let v = ExtractedValue {
            name: "test".to_string(),
            raw: todo!(),     // Replace with valid A2lValue after confirming the type
            physical: PhysicalValue::Numeric(42.0),
            unit: Some("mm".to_string()),
        };
        let block = extracted_value_to_block(&v, "Test desc");
        match block {
            Block::Constant(c) => {
                assert_eq!(c.name, "test");
                assert_eq!(c.value, Value::WERT(vec![42.0]));
                assert_eq!(c.attrs.iter().find(|a| a.identifier == "LANGNAME").unwrap().value, "Test desc");
                assert_eq!(c.attrs.iter().find(|a| a.identifier == "EINHEIT_W").unwrap().value, "mm");
            }
            _ => panic!("Expected Constant"),
        }
    }

    #[test]
    fn test_extracted_curve_skip_verbal() {
        let c = ExtractedCurve {
            name: "bad_curve".to_string(),
            x_axis: vec![1.0, 2.0],
            x_unit: Some("rpm".to_string()),
            values: vec![PhysicalValue::Numeric(1.0), PhysicalValue::Verbal("BAD".into())],
            unit: Some("Nm".to_string()),
        };
        let result = extracted_curve_to_block(&c, "desc");
        assert!(result.is_none(), "Curve with verbal values should be skipped");
    }

    #[test]
    fn test_extracted_valblk_mixed_to_text() {
        let vb = ExtractedValBlk {
            name: "mixed".to_string(),
            values: vec![
                PhysicalValue::Numeric(1.0),
                PhysicalValue::Verbal("TWO".into()),
                PhysicalValue::Numeric(3.0),
            ],
            unit: Some("deg".to_string()),
        };
        let block = extracted_valblk_to_block(&vb, "desc");
        match block {
            Block::ConstantBlock(c) => {
                assert_eq!(c.name, "mixed");
                assert_eq!(c.value, Value::TEXT(vec!["1".into(), "TWO".into(), "3".into()]));
            }
            _ => panic!("Expected ConstantBlock"),
        }
    }
}
```

- [ ] **Step 4: Run unit tests**

```bash
cargo test gen::tests -- --nocapture
```

Expected: All 3 gen unit tests pass.

- [ ] **Step 5: Write integration test in main.rs test section**

Add to the existing `#[cfg(test)] mod tests` block at the bottom of `main.rs`:

```rust
#[rstest]
fn test_gen_basic() {
    let a2l_path = std::path::Path::new("./test-dcms/simple_test.a2l");
    let hex_path = std::path::Path::new("./test-dcms/simple_test.hex");
    let dcm_data = dcm_utils::gen::gen_dcm_data(a2l_path, hex_path)
        .expect("gen_dcm_data should succeed");

    // Check VALUE → FESTWERT
    assert!(dcm_data.contains_block("test_scalar"));
    let block = dcm_data.blocks.get("test_scalar").unwrap();
    let values = block.get_values().try_into_f64().unwrap();
    assert!((values[0] - 42.0).abs() < 0.001);

    // Check CURVE → GRUPPENKENNLINIE
    assert!(dcm_data.contains_block("test_curve"));

    // Check VAL_BLK → FESTWERTEBLOCK
    assert!(dcm_data.contains_block("test_valblk"));

    // Check that derived axis block was created for test_curve
    assert!(dcm_data.contains_block("test_curve_X"));
}
```

- [ ] **Step 6: Run the integration test**

```bash
cargo test test_gen_basic -- --nocapture
```

Expected: Test compiles and passes. The A2L+HEX fixtures parse correctly, producing FESTWERT (value 42.0), GRUPPENKENNLINIE, and FESTWERTEBLOCK blocks with derived axis distribution.

- [ ] **Step 7: Run full test suite to check for regressions**

```bash
cargo test
```

Expected: All existing tests still pass alongside the new tests.

- [ ] **Step 8: Run clippy for code quality**

```bash
cargo clippy
```

Expected: No warnings or errors.

- [ ] **Step 9: Commit**

```bash
git add test-dcms/simple_test.a2l test-dcms/simple_test.hex src/main.rs src/gen.rs
git commit -m "test: add integration and unit tests for gen command"
```

---

### Task 6: Full build verification and final commit

- [ ] **Step 1: Release build**

```bash
cargo build --release
```

Expected: Clean release build with no errors.

- [ ] **Step 2: Verify CLI help shows gen command**

```bash
cargo run -- --help
cargo run -- gen --help
```

Expected: `gen` appears as a subcommand with `--a2l`, `--hex`, `--output` options.

- [ ] **Step 3: Commit any final changes**

```bash
git status
git diff --stat
# Only add changed source files, never use -A
git add src/ test-dcms/
git diff --staged
git commit -m "chore: final verification of gen command implementation"
```

---

## Summary of Changes

| File | Action | Lines |
|------|--------|-------|
| `Cargo.toml` | Modify: add 2 deps | +2 |
| `src/main.rs` | Modify: add import + Gen variant + match arm | ~+25 |
| `src/lib.rs` | Modify: add pub mod gen | +1 |
| `src/gen.rs` | Create: full conversion module | ~200 |
| `test-dcms/simple_test.a2l` | Create: test fixture | ~30 |
| `test-dcms/simple_test.hex` | Create: test fixture | ~3 |

## Key Dependencies

- `a2ldeser` (git): Extractor, ExtractedObject, PhysicalValue, HexMemory
- `a2lfile` v3.3: Module, load()

## Edge Cases Covered

1. **Verbal values in CURVE/MAP**: Entire block skipped with stderr warning
2. **Mixed Verbal/Numeric in ValBlk**: Entire block converted to TEXT
3. **Axis name collision**: Standalone AXIS_PTS wins, derived axis skipped with warning
4. **Missing long_identifier**: Falls back to characteristic name as LANGNAME
5. **Zero successful extractions**: Returns empty DcmData, still writes output file
6. **HEX/A2L parse failure**: Returns `Err` with descriptive message
