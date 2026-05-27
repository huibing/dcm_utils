# Diff Multi-Source Comparison Design

## Goal

Extend the `diff` CLI command to compare calibration values across different source formats: DCM files and A2L+HEX pairs. The diff engine itself does not change — both sources normalize to `DcmData`, then the existing comparison logic runs.

## Command Syntax

Three modes, all using the same flags:

```bash
# DCM vs DCM
dcm_utils diff --dcm a.DCM --dcm b.DCM -o diff.json

# DCM vs A2L+HEX (DCM entries ordered first, then A2L+HEX pairs)
dcm_utils diff --dcm ref.DCM --a2l cal.a2l -x flash.hex -o diff.json

# A2L+HEX vs A2L+HEX
dcm_utils diff --a2l v1.a2l -x v1.hex --a2l v2.a2l -x v2.hex -o diff.json
```

### CLI Definition (main.rs)

```rust
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

All three fields are `Vec<PathBuf>` — clap's derive API collects repeated flags into vectors.

### CLI Validation Logic

Clap cannot express cross-flag constraints natively, so a `validate_and_build_sources` function in `diff.rs` handles validation before any I/O:

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
    // sources is ordered canonically: DCM entries first, then A2L+HEX pairs
    let right = sources.pop().unwrap();
    let left = sources.pop().unwrap();
    Ok((left, right))
}
```

### Match Arm (main.rs)

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

    // Print summary (uses renamed fields: left_label / right_label)
    println!("{}", "=== Calibration Diff Results ===".bold());
    println!("Left:  {}", result.metadata.left_label.cyan());
    println!("Right: {}", result.metadata.right_label.cyan());
    // ... rest of summary output unchanged ...

    // Write JSON
    let json = serde_json::to_string_pretty(&result).unwrap();
    std::fs::write(&output, json).expect("Failed to write diff output");
    println!("Diff details written to: {}", output.display().to_string().blue());
},
```

### CLI Flags

| Flag | Description |
|---|---|
| `--dcm <PATH>` | A DCM file source (repeatable, one per side) |
| `--a2l <PATH>` | An A2L calibration description (paired with `--hex` on the same side) |
| `-x, --hex <PATH>` | An Intel HEX flash image (paired with `--a2l` on the same side) |
| `-o, --output <PATH>` | Output JSON file (default: `diff.json`) |

Backward compatibility with the old positional-args `diff original.DCM modified.DCM` is intentionally **broken**.

## Architecture

### Data Flow

```
CLI args → CalSource::load() → DcmData ─┐
                                         ├→ dcm_diff_with_metadata() → DcmDiffResult → JSON
CLI args → CalSource::load() → DcmData ─┘
```

### New Type: `CalSource`

Added in `src/diff.rs`:

```rust
#[derive(Debug, Clone)]
pub enum CalSource {
    Dcm(PathBuf),
    A2lHex { a2l: PathBuf, hex: PathBuf },
}
```

Methods:

- `load(&self) -> Result<DcmData, Box<dyn Error>>` — for DCM sources, wraps `DcmData::new()` in a `catch_unwind` to convert panics (missing file, corrupt content) into `Err`, giving users a clean error message instead of a panic backtrace. For A2L+HEX sources, delegates to `gen::gen_dcm_data()`.
- `label(&self) -> String` — uses `path.display().to_string()` consistently for both variants. A2L+HEX format: `"<a2l_path> + <hex_path>"` (space-padded `+` separator, full paths).
- `label(&self) -> String` — uses `path.display().to_string()` consistently for both variants. A2L+HEX format: `"<a2l_path> + <hex_path>"` (space-padded `+` separator, full paths).

### Updated Function Signature

`dcm_diff_with_metadata` changes from `(&Path, &Path)` to `(&CalSource, &CalSource)`:

```rust
pub fn dcm_diff_with_metadata(
    left: &DcmData,
    right: &DcmData,
    left_src: &CalSource,
    right_src: &CalSource,
) -> DcmDiffResult
```

`DiffMetadata::new()` changes to accept two `&str` labels (extracted via `CalSource::label()`) instead of `&Path`, and the Rust fields are renamed from `original_file`/`modified_file` to `left_label`/`right_label`. The serde keys are preserved for backward compatibility via `#[serde(rename)]`:

```rust
pub struct DiffMetadata {
    #[serde(rename = "original_file")]
    pub left_label: String,
    #[serde(rename = "modified_file")]
    pub right_label: String,
    pub timestamp: String,
}
```

This ensures existing JSON consumers see the same `"original_file"` / `"modified_file"` keys.

`diff.rs` gains `use crate::gen::gen_dcm_data;` for the A2L+HEX load path.

## File Changes

| File | Change |
|---|---|
| `src/diff.rs` | Add `CalSource` enum with `load()`/`label()`. Add `validate_and_build_sources()`. Change `DiffMetadata::new()` to take two `&str`, rename fields with `#[serde(rename)]`. Update `dcm_diff_with_metadata` signature. Add `use crate::gen::gen_dcm_data`. |
| `src/main.rs` | Replace positional `Diff` variant with `Vec`-based flag args. Update doc comments to show new flag-based examples. Call `validate_and_build_sources()` in match arm, call `load()` on each source, pass to `dcm_diff_with_metadata`. Update field access to `left_label`/`right_label`. |
| `src/lib.rs` | Add `CalSource`, `validate_and_build_sources` to the `pub use diff::{...}` re-export line. |
| `tests/*.rs` (3 files) | Update callers of `dcm_diff_with_metadata` in `test_diff_enhanced_output.rs`, `test_diff_2d_map_comprehensive.rs`, `test_diff_map_refactor.rs` to wrap paths in `CalSource::Dcm(...)`. The other 2 test files call only `dcm_diff()` (unchanged). |
| `tests/*.rs` (all files) | Scan and update any assertions referencing the old `original_file`/`modified_file` Rust field names (renamed to `left_label`/`right_label`). JSON key assertions are unchanged due to `#[serde(rename)]`. |

## Error Handling

- **DCM file missing/corrupt**: `CalSource::load()` wraps `DcmData::new()` in `catch_unwind` to convert panics into `Err`, producing a clean error message like the A2L+HEX path
- **A2L/HEX file parse failure**: `gen_dcm_data` returns `Err` → hard error, diff exits
- **Individual characteristic extraction failure**: handled internally by `gen_dcm_data` via stderr summary; diff proceeds with successfully extracted blocks
- **Invalid source count** (any count not equal to 2): manual validation in the match arm prints error and exits before any I/O
- **Unpaired `--a2l`/`--hex`** (`a2l.len() != hex.len()`): manual validation in the match arm prints error and exits before any I/O

## Testing

### Unit Tests (src/diff.rs)

- `test_cal_source_label_dcm` — verify label uses full `display()` path
- `test_cal_source_label_a2l_hex` — verify label format `"<a2l> + <hex>"`
- `test_cal_source_validation` — unit-test the validation function: reject any source count not equal to 2 (0, 1, 3, 4), and reject mismatched a2l/hex counts

### Integration Tests (tests/ directory)

- **`test_diff_dcm_vs_dcm`** — `CalSource::Dcm(a)` vs `CalSource::Dcm(b)` using two known-different test DCM files, verify `DcmDiffResult` has expected diff count
- **`test_diff_a2l_vs_a2l_same`** — generate `DcmData` from same A2L+HEX twice in memory (no file I/O), diff them, expect zero differences
- **`test_diff_a2l_vs_a2l_different`** — generate from two different A2L+HEX pairs, verify non-zero differences are detected
- **`test_diff_a2l_vs_dcm`** — compare in-memory gen result against `DcmData::new()` of the gen output file, expect zero differences (round-trip)
- **`test_diff_a2l_vs_dcm_different`** — compare gen result against a different DCM file, verify differences detected

### Test Data

- Reuses existing `test-dcms/simple_test.a2l` and `test-dcms/simple_test.hex` for A2L+HEX sources
- Reuses existing `test-dcms/` DCM files for DCM sources
- No new test fixture files required
- Extraction-failure resilience is already tested by existing `gen.rs` unit tests (verbal value skipping, etc.) and exercised at runtime by `gen_dcm_data`'s stderr summary

### Updating Existing Tests

The following files call `dcm_diff_with_metadata` and need mechanical updates:

| File | Changes |
|---|---|
| `tests/test_diff_enhanced_output.rs` | Wrap path args in `CalSource::Dcm(...)`. Update field access: `.original_file` → `.left_label`, `.modified_file` → `.right_label`. JSON string assertions on `"original_file"`/`"modified_file"` keys unchanged (serde rename preserves them). |
| `tests/test_diff_2d_map_comprehensive.rs` | Wrap path args in `CalSource::Dcm(...)`. Update field access: `.original_file` → `.left_label`, `.modified_file` → `.right_label`. |
| `tests/test_diff_map_refactor.rs` | Wrap path args in `CalSource::Dcm(...)`. |
| `src/main.rs` | `result.metadata.original_file` → `result.metadata.left_label`, `result.metadata.modified_file` → `result.metadata.right_label`. |

The 2 test files that call only `dcm_diff()` (`test_diff_json_output.rs`, `test_diff_table_changes.rs`) are unchanged.
