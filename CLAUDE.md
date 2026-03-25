# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust CLI tool for parsing and manipulating DCM (DAMOS Calibration Memory) files used in automotive ECU calibration. It supports reading, merging, updating, and filtering DCM files containing calibration parameters.

## Build, Test, and Development Commands

### Build
```bash
cargo build              # Debug build
cargo build --release    # Release build
```

### Test
```bash
cargo test               # Run all tests
cargo test <test_name>   # Run a specific test (e.g., cargo test test_festwert)
cargo test -- --nocapture  # Run tests with stdout output visible
```

### Lint and Format
```bash
cargo clippy             # Run clippy (configured with .clippy.toml)
cargo fmt                # Format code
```

### Run the CLI
```bash
cargo run -- merge file1.DCM file2.DCM -o merged.DCM
cargo run -- update base.DCM updates.DCM -o updated.DCM
cargo run -- filter input.DCM --include "pattern1" "pattern2" -o filtered.DCM
```

## Architecture Overview

### Core Data Flow

1. **Parsing**: `DcmData::new(path)` reads a DCM file and parses it into `Block` variants
2. **Storage**: Blocks are stored in `IndexMap<String, Block>` (preserves insertion order, keyed by variable name)
3. **Manipulation**: Operations like `merge_dcm_data`, `update_dcm_data` modify the block map
4. **Output**: `write_dcm_data()` uses Handlebars templates (`templates/dcm_template.hbs`) to render DCM files

### Module Structure

```
src/
├── lib.rs           # Main library: DcmData struct, merge/update operations, file I/O
├── main.rs          # CLI entry point with clap subcommands (merge, update, filter)
├── block.rs         # Block enum: unified interface for all block types
├── value.rs         # Value enum: WERT (f64 values) or TEXT (string values)
├── diff.rs          # DCM file comparison (DcmDiff enum and dcm_diff function)
├── blocks/          # Block type implementations
│   ├── festwert.rs              # FESTWERT (single constant)
│   ├── festwerteblock.rs        # FESTWERTEBLOCK (constant array)
│   ├── gruppenkennlinie.rs      # GRUPPENKENNLINIE (1D table/curve)
│   ├── stuetzstellenverteilung.rs # STUETZSTELLENVERTEILUNG (axis breakpoints)
│   └── gruppenkennfeld.rs       # GRUPPENKENNFELD (2D map)
└── attr/            # Attribute parsing
    ├── string_attr.rs   # String attributes (LANGNAME, EINHEIT_W/X/Y)
    ├── value_attr.rs    # Value attributes (WERT, ST/X, ST/Y, TEXT)
    └── attr_arbitor.rs  # Attr enum dispatcher
```

### Block Types (German ASAM terminology)

| Block Type | Description | Key Fields |
|------------|-------------|------------|
| `FESTWERT` | Single constant value | `name`, `value: Value`, `attrs` |
| `FESTWERTEBLOCK` | Constant array (1D) | `name`, `dim`, `value: Value` |
| `GRUPPENKENNLINIE` | 1D lookup table/curve | `name`, `dim`, `axis`, `axis_var_name`, `value` |
| `STUETZSTELLENVERTEILUNG` | Axis breakpoint distribution | `name`, `dim`, `value` (X-axis values) |
| `GRUPPENKENNFELD` | 2D map/table | `name`, `dim: (x, y)`, `x_axis`, `y_axis`, `value: Vec<Value>` |

### Parsing Pattern

Each block type implements `FromStr`:
```rust
impl FromStr for FESTWERT {
    type Err = Box<dyn Error>;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Parse lines using Attr arbiter
        // Lines dispatch to StringAttr or ValueAttr
    }
}
```

The `Attr` enum in `attr_arbitor.rs` dispatches lines to:
- `StringAttr`: Metadata like `LANGNAME`, `EINHEIT_W`, `EINHEIT_X`, `EINHEIT_Y`
- `ValueAttr`: Data lines `WERT`, `ST/X`, `ST/Y`, `TEXT`
- `AxisVar`: Axis references `*SSTX`, `*SSTY`

### Key Dependencies

- `indexmap`: Preserves insertion order for blocks (required for consistent output)
- `handlebars`: Template engine for DCM file generation
- `serde_json`: JSON serialization for template data
- `rstest`: Parametrized testing framework
- `clap`: CLI argument parsing
- `regex`: Pattern matching for filter command

## Testing Notes

- Tests use sample DCM files in `./test-dcms/`
- Output files are written to `./output/` (created automatically in debug builds)
- Tests include Intel HEX file parsing experiments (`test_ihex*`)
- The `approx` crate is used for floating-point comparisons

## Custom Handlebars Helper

The `dcm_vector` helper in `lib.rs` formats arrays for DCM output:
- Chunks values 6 per line
- Switches between `WERT` and `TEXT` identifiers based on content
- Formats floats with 16 decimal places
