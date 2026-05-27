# Diff Multi-Source Comparison Design

## Goal

Extend the `diff` CLI command to compare calibration values across different source formats: DCM files and A2L+HEX pairs. The diff engine itself does not change — both sources normalize to `DcmData`, then the existing comparison logic runs.

## Command Syntax

Three modes, all using the same flags:

```bash
# DCM vs DCM
dcm_utils diff --dcm a.DCM --dcm b.DCM -o diff.json

# A2L+HEX vs DCM
dcm_utils diff --a2l cal.a2l -x flash.hex --dcm ref.DCM -o diff.json

# A2L+HEX vs A2L+HEX
dcm_utils diff --a2l v1.a2l -x v1.hex --a2l v2.a2l -x v2.hex -o diff.json
```

### CLI Flags

| Flag | Description |
|---|---|
| `--dcm <PATH>` | A DCM file source (repeatable, one per side) |
| `--a2l <PATH>` | An A2L calibration description (paired with `--hex` on the same side) |
| `-x, --hex <PATH>` | An Intel HEX flash image (paired with `--a2l` on the same side) |
| `-o, --output <PATH>` | Output JSON file (default: `diff.json`) |

### Validation Rules

- Exactly 2 sources must be specified (2× `--dcm`, or `--dcm` + `--a2l`+`--hex`, or 2× `--a2l`+`--hex`)
- A2L and HEX must come in pairs — `--a2l` without `--hex` on the same side is rejected, and vice versa
- First source encountered becomes "left" (original), second becomes "right" (modified)
- Backward compatibility with the old positional-args `diff original.DCM modified.DCM` is intentionally **broken**

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
enum CalSource {
    Dcm(PathBuf),
    A2lHex { a2l: PathBuf, hex: PathBuf },
}
```

Methods:

- `load(&self) -> Result<DcmData, Box<dyn Error>>` — dispatches to `DcmData::new()` or `gen::gen_dcm_data()`
- `label(&self) -> String` — human-readable label for metadata (e.g., `"cal.a2l+flash.hex"` for A2L+HEX, or the DCM path)

### Updated Function Signature

`dcm_diff_with_metadata` changes from taking `&Path` to taking `&CalSource`:

```rust
pub fn dcm_diff_with_metadata(
    left: &DcmData,
    right: &DcmData,
    left_src: &CalSource,
    right_src: &CalSource,
) -> DcmDiffResult
```

`DiffMetadata` uses `CalSource::label()` for `original_file` and `modified_file`.

## File Changes

| File | Change |
|---|---|
| `src/diff.rs` | Add `CalSource` enum with `load()`/`label()`. Update `dcm_diff_with_metadata` signature and `DiffMetadata` construction. |
| `src/main.rs` | Replace positional `Diff` variant with flag-based args. Add parse-side validation (pair counts, exactly 2 sources). Update match arm to use `CalSource`. |

## Error Handling

- **A2L/HEX file parse failure**: hard error, diff exits immediately (files are truly broken)
- **Individual characteristic extraction failure**: handled internally by `gen_dcm_data` via stderr summary; diff proceeds with successfully extracted blocks
- **Invalid source count**: clap-level validation rejects before any I/O (0, 1, or 3+ sources)
- **Unpaired --a2l/--hex**: rejected at argument parsing time

## Testing

- **Unit tests** in `diff.rs`: `CalSource::label()` for both variants
- **Integration test**: `test_diff_a2l_vs_dcm` — generate DCM from `simple_test.a2l`+`simple_test.hex`, load a DCM file from that same gen output, diff them, expect zero differences (round-trip)
- **Integration test**: `test_diff_a2l_vs_a2l` — diff the same A2L+HEX against itself, expect zero differences
- **Integration test**: `test_diff_dcm_vs_dcm` — verify flag-based DCM-vs-DCM works the same as the old positional syntax
- **Edge case**: A2L source where a characteristic fails extraction — diff still completes comparing remaining blocks (test with a deliberately-broken A2L characteristic)
