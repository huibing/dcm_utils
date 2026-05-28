# Design: `--approx` Flag for Diff Command

## Summary

Add a `--approx` CLI flag to the `diff` subcommand. When enabled, floating-point value comparisons use relative tolerance instead of exact equality, omitting diffs caused by float representation noise. A warning is printed when the flag is active.

## Motivation

DCM files store calibration values as f64, but values may differ between sources due to float round-trip precision (e.g., f32->f64 conversion from HEX extraction vs. DCM text parsing). These are not meaningful calibration changes and should be suppressible.

## Design

### CLI

Add `--approx` flag to the `Diff` subcommand in `main.rs`:

```rust
/// Use approximate comparison for floating-point values (relative tolerance 1e-8)
#[arg(long, default_value_t = false)]
approx: bool,
```

Default is `false` (exact equality), preserving existing behavior.

### Diff Function Signature

Extend `dcm_diff_with_details` and `dcm_diff_with_metadata` to accept an `approx: bool` parameter. The public `dcm_diff` wrapper always passes `approx: false` to maintain backward compatibility for library callers.

### Comparison Methods

Add `approx_eq` methods to `Value` and `Block` alongside the existing `PartialEq` impls. This keeps default `==` behavior unchanged and isolates approximate logic.

**`Value::approx_eq`** (`value.rs`):
- `WERT` vs `WERT`: use `approx::relative_eq!` with `max_relative = 1e-8` and `epsilon = 1e-12` (the epsilon handles near-zero values where relative comparison is undefined)
- `TEXT` vs `TEXT`: exact string equality (unchanged)
- Mixed types: false (unchanged)
- Length mismatch: false (unchanged)

**`Block::approx_eq`** (`block.rs`):
- Same structure as `PartialEq`, but calls `Value::approx_eq` instead of `==`
- Axis vectors (`Vec<f64>`) use a shared helper `fn approx_eq_f64_slice(a: &[f64], b: &[f64]) -> bool` that applies the same `relative_eq!(max_relative = 1e-8, epsilon = 1e-12)` element-wise
- Map variant: include `x_axis_name` and `y_axis_name` string comparison (fixing the existing `PartialEq` omission where these are not checked in `Block::PartialEq`)

### Warning Log

When `--approx` is active, log a warning at the start of diff processing:
```
WARN - Approximate comparison enabled: float differences within relative tolerance 1e-8 (epsilon 1e-12) will be treated as equal
```

### Edge Cases

- **Near-zero values**: `relative_eq!` alone is undefined for zero. The `epsilon = 1e-12` parameter handles this: two values within `1e-12` absolute difference are treated as equal regardless of magnitude. This covers f32-to-f64 conversion noise for zero and near-zero calibration values.
- **NaN/Inf**: `approx::relative_eq!` returns `false` for any comparison involving NaN. This is acceptable behavior — NaN in DCM calibration data is invalid, so treating NaN != anything as a diff is correct. No special handling needed.
- **Empty vectors**: `Value::WERT(vec![])` compared with another empty WERT returns `true`. Unequal-length vectors return `false`.

### Call Path

```
main.rs: Diff { approx, ... }
  -> dcm_diff_with_metadata(&left, &right, &left_src, &right_src, approx)
    -> dcm_diff_with_details(left, right, detailed, approx)
      -> if approx { left_block.approx_eq(right_block) } else { left_block == right_block }
```

## What Stays the Same

- Default behavior (no `--approx`): exact `==` comparison, no changes
- `PartialEq` impls on `Value` and `Block` are untouched
- `generate_change_description` uses approximate comparison for Value/axis fields when `--approx` is active, so it won't report "values changed" for float-noise differences the user chose to suppress

## Dependencies

- Move `approx = "0.5.1"` from `[dev-dependencies]` to `[dependencies]` in `Cargo.toml` since `approx_eq` methods live in library code (`value.rs`, `block.rs`)

## Testing

- Unit test `Value::approx_eq` with near-equal and clearly-different f64 pairs
- Unit test `Block::approx_eq` for each block type
- Integration test: diff two DCM files with and without `--approx`, verify fewer differences reported with `--approx` when only float noise differs
