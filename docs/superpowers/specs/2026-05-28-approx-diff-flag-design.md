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

Extend `dcm_diff_with_details` and `dcm_diff_with_metadata` to accept an `approx: bool` parameter. Update the public `dcm_diff` wrapper accordingly.

### Comparison Methods

Add `approx_eq` methods to `Value` and `Block` alongside the existing `PartialEq` impls. This keeps default `==` behavior unchanged and isolates approximate logic.

**`Value::approx_eq`** (`value.rs`):
- `WERT` vs `WERT`: use `approx::relative_eq!` with `max_relative = 1e-8`
- `TEXT` vs `TEXT`: exact string equality (unchanged)
- Mixed types: false (unchanged)

**`Block::approx_eq`** (`block.rs`):
- Same structure as `PartialEq`, but calls `Value::approx_eq` instead of `==`
- Axis vectors (f64) also use relative comparison

### Warning Log

When `--approx` is active, log a warning at the start of diff processing:
```
WARN - Approximate comparison enabled: float differences within relative tolerance 1e-8 will be treated as equal
```

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
- `generate_change_description` still uses `!=` for string-level change descriptions (these are informational, not gatekeeping)

## Dependencies

- `approx` crate is already a dependency (currently dev-only for tests). Move or keep it as a full dependency since it's now used in library code.

## Testing

- Unit test `Value::approx_eq` with near-equal and clearly-different f64 pairs
- Unit test `Block::approx_eq` for each block type
- Integration test: diff two DCM files with and without `--approx`, verify fewer differences reported with `--approx` when only float noise differs
