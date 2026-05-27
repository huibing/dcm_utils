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

### CLI Validation Logic (manual, in the match arm)

Clap cannot express cross-flag constraints natively, so validation happens in the match arm before any I/O:

1. **Count check**: `dcm.len() + a2l.len()` must equal exactly 2. Fewer or more sources → reject with a clear error message.
2. **Pairing check**: `a2l.len() == hex.len()`. Extra A2Ls without HEXs (or vice versa) → reject.
3. **Source construction** (canonical order):
   - Each `--dcm` becomes `CalSource::Dcm(path)`
   - Each `--a2l`/`--hex` pair (zipped by index) becomes `CalSource::A2lHex { a2l, hex }`
   - Sources are ordered canonically: `--dcm` entries first (in order), then `--a2l`/`--hex` pairs (in order). Note: clap `Vec` does not preserve interleaving across different flags, so `--a2l a.a2l -x a.hex --dcm b.DCM` still produces DCM first. Examples above use the canonical order.
4. **First source = left** (original), **second source = right** (modified)

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
pub enum CalSource {
    Dcm(PathBuf),
    A2lHex { a2l: PathBuf, hex: PathBuf },
}
```

Methods:

- `load(&self) -> Result<DcmData, Box<dyn Error>>` — dispatches to `DcmData::new()` (wrapping the panic-prone call in `Ok()`) or `gen::gen_dcm_data()`. Note: `DcmData::new()` currently panics on I/O errors; the `Result` return type covers the A2L+HEX path and leaves room for a future fallible `DcmData::try_new()` without changing the public API.
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

`DiffMetadata::new()` changes to accept two `&str` labels (extracted via `CalSource::label()`) instead of `&Path`, and the fields are renamed from `original_file`/`modified_file` to `left_label`/`right_label` to accurately describe composite A2L+HEX sources. This avoids coupling `DiffMetadata` to `CalSource`.

The old signature is **removed** — the 3 existing integration tests under `tests/` that call `dcm_diff_with_metadata` will be updated to construct `CalSource::Dcm(path)` wrappers. `CalSource` is re-exported from `lib.rs` (`pub use diff::CalSource;`).

`diff.rs` gains `use crate::gen::gen_dcm_data;` for the A2L+HEX load path.

## File Changes

| File | Change |
|---|---|
| `src/diff.rs` | Add `CalSource` enum with `load()`/`label()`. Change `DiffMetadata::new()` to take two `&str`. Update `dcm_diff_with_metadata` signature. Add `use crate::gen::gen_dcm_data`. |
| `src/main.rs` | Replace positional `Diff` variant with `Vec`-based flag args. Update doc comments to show new flag-based examples. Add manual validation (count + pairing) in the match arm before any I/O. Build two `CalSource` values, call `load()` on each, pass to `dcm_diff_with_metadata`. |
| `src/lib.rs` | Add `CalSource` to the `pub use diff::{...}` re-export line. |
| `tests/*.rs` (5 files) | Update all callers of `dcm_diff_with_metadata` to wrap paths in `CalSource::Dcm(...)`. |

## Error Handling

- **DCM file missing/corrupt**: `DcmData::new()` panics (existing behavior; covered by the fact that cal files should be valid at this point in the pipeline)
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
- No new test fixture files required for the core tests
- Edge case with broken characteristic: create `test-dcms/broken_char.a2l` with a CHARACTERISTIC referencing a nonexistent RECORD_LAYOUT, paired with a minimal valid HEX; verify diff still completes

### Updating Existing Tests

All 5 existing integration tests that call `dcm_diff_with_metadata(..., &original, &modified)` will be updated to pass `&CalSource::Dcm(original.to_path_buf())` etc. This is a mechanical change.
