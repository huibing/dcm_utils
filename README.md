# DCM Utils

A Rust-based command-line tool for parsing, manipulating, and comparing DCM (DAMOS Calibration Memory) files used in automotive ECU calibration.

## Overview

DCM Utils is a powerful utility designed for automotive engineers working with ECU calibration data. It provides comprehensive support for reading, merging, updating, filtering, and comparing DCM files in the ASAM DAMOS format.

## Features

- **📖 Parse DCM Files**: Read and parse DCM files with support for all standard block types
- **🔀 Merge**: Combine multiple DCM files, with the first file as the base
- **🔄 Update**: Apply calibration changes from one or more DCM files to a base file
- **🔍 Filter**: Include or exclude variables using regex patterns
- **📊 Diff**: Compare two DCM files and identify differences
- **🔧 Gen**: Generate DCM files from A2L calibration descriptions and Intel HEX flash images
- **📝 Output**: Generate well-formatted DCM files using Handlebars templates

## Supported DCM Block Types

| Block Type | Description |
|------------|-------------|
| `FESTWERT` | Single constant value |
| `FESTWERTEBLOCK` | Constant array (1D) |
| `GRUPPENKENNLINIE` | 1D lookup table/curve |
| `STUETZSTELLENVERTEILUNG` | Axis breakpoint distribution |
| `GRUPPENKENNFELD` | 2D map/table |

## Installation

### Prerequisites

- Rust toolchain (version 1.94.0 or later)
- Cargo package manager

### Build from Source

```bash
# Clone the repository
git clone git@github.com:huibing/dcm_utils.git
cd dcm_utils

# Build in debug mode
cargo build

# Build in release mode (optimized)
cargo build --release
```

The compiled binary will be available at:
- Debug: `target/debug/dcm_utils`
- Release: `target/release/dcm_utils`

## Usage

### Command Overview

```bash
dcm_utils <COMMAND> [OPTIONS]
```

Available commands:
- `merge` - Merge multiple DCM files
- `update` - Update a DCM file with data from other files
- `filter` - Filter variables by regex patterns
- `diff` - Compare two DCM files
- `diff-base` - Compare a base source against multiple other sources
- `gen` - Generate DCM file from A2L and HEX calibration files
- `help` - Show help information

### Merge Command

Merge multiple DCM files into one, using the first file as the base.

```bash
dcm_utils merge file1.DCM file2.DCM file3.DCM -o merged.DCM
```

**Behavior:**
- Variables from the first file are kept as the base
- Variables from subsequent files are added if they don't exist in the base
- Existing variables are not overwritten (use `update` for that)

**Options:**
- `-o, --output <OUTPUT>` - Output file path (default: `merged.dcm`)

### Update Command

Update the first DCM file with data from other DCM files.

```bash
dcm_utils update base.DCM updates.DCM -o updated.DCM
```

**Behavior:**
- Only variables that exist in the base file are updated
- New variables from update files are discarded
- Multiple update files can be specified

**Options:**
- `-o, --output <OUTPUT>` - Output file path (default: `updated.dcm`)

### Filter Command

Filter DCM variables using include or exclude regex patterns.

```bash
# Include only variables matching patterns
dcm_utils filter input.DCM --include "VAR.*" "CFG.*" -o filtered.DCM

# Exclude variables matching patterns
dcm_utils filter input.DCM --exclude "Temp.*" "Test.*" -o filtered.DCM
```

**Options:**
- `-i, --include <PATTERNS>` - Include only matching variables
- `-e, --exclude <PATTERNS>` - Exclude matching variables
- `-o, --output <OUTPUT>` - Output file path (default: `filtered.dcm`)

**Note:** Either `--include` or `--exclude` must be provided, but not both.

### Diff-Base Command

Compare variables from a base calibration source against multiple other sources. Only variables present in the base source are compared; variables not in the base are ignored. Each variable is compared across all sources using f32 byte-level comparison.

```bash
# Compare base DCM against two other DCM files
dcm_utils diff-base --base base.DCM --other source1.DCM --other source2.DCM

# Compare A2L+HEX base against DCM files
dcm_utils diff-base --base --a2l cal.a2l -x flash.hex --other modified.DCM
```

**Output:**
- Console summary listing total variables and those with differences
- JSON file with per-variable, per-source values
- Optional web page (`--web`) with labeled sources (Base, Source 1, Source 2, ...)

**Options:**
- `--base <PATH>` - Base DCM file
- `--base-a2l <PATH>` - Base A2L file (paired with `--base-hex`)
- `--base-hex <PATH>` - Base Intel HEX image (paired with `--base-a2l`)
- `--other <PATH>` - Other DCM file (repeatable)
- `--other-a2l <PATH>` - Other A2L file (paired with `--other-hex`, repeatable)
- `--other-hex <PATH>` - Other Intel HEX image (paired with `--other-a2l`)
- `-o, --output <OUTPUT>` - Output JSON file path (default: `diff-base.json`)
- `--web` - Serve results as a web page

### Diff Command

Compare calibration data across DCM files and A2L+HEX pairs. The diff engine normalizes all sources to `DcmData` before comparison, enabling cross-format diff.

```bash
# DCM vs DCM
dcm_utils diff --dcm original.DCM --dcm modified.DCM -o diff.json

# DCM vs A2L+HEX (DCM ordered first)
dcm_utils diff --dcm ref.DCM --a2l calibration.a2l -x flash.hex -o diff.json

# A2L+HEX vs A2L+HEX
dcm_utils diff --a2l v1.a2l -x v1.hex --a2l v2.a2l -x v2.hex -o diff.json
```

**Floating-point comparison:**
- Values are converted to single-precision (f32) and compared byte-by-byte
- This means two values that differ only below f32 precision (e.g., `1.0` vs `1.0 + 1e-9`) are treated as equal
- This matches how calibration data is actually stored in ECU flash memory

**Output:**
- Console summary with color-coded statistics
- JSON file with detailed differences:
  - `New` - Variables present only in the right source
  - `Deleted` - Variables present only in the left source
  - `Changed` - Variables with different values
  - `ChangedMap` - 2D maps with differences (full JSON representation)

**Options:**
- `--dcm <PATH>` - A DCM file source (repeatable, one per side)
- `--a2l <PATH>` - An A2L calibration description (paired with `--hex` on the same side)
- `-x, --hex <PATH>` - An Intel HEX flash image (paired with `--a2l` on the same side)
- `-o, --output <OUTPUT>` - Output JSON file path (default: `diff.json`)
- `--web` - Serve diff results as a web page

**Example Output:**
```
=== Calibration Diff Results ===
Left:  test-dcms/test1.DCM
Right: test-dcms/simple_test.a2l + test-dcms/simple_test.hex
Timestamp: 1779877997

New blocks: 4
Deleted blocks: 10
Changed blocks: 0
Total differences: 14

Diff details written to: diff.json
```

### Gen Command

Generate a DCM file from A2L calibration descriptions and an Intel HEX flash image.

```bash
dcm_utils gen --a2l calibration.a2l -x flash.hex -o all_cali.DCM
```

**Behavior:**
- Extracts all calibration characteristics from the A2L file and HEX binary
- Maps A2L types to DCM block types (VALUE→FESTWERT, CURVE→GRUPPENKENNLINIE+axis, MAP→GRUPPENKENNFELD+axes, VAL_BLK→FESTWERTEBLOCK, ASCII→FESTWERT)
- Failed extractions are skipped with a summary printed to stderr
- CURVE/MAP blocks with verbal (non-numeric) values are skipped with a warning
- Mixed numeric/verbal VAL_BLK blocks are converted entirely to TEXT format

**Options:**
- `-a, --a2l <A2L>` - Path to the A2L calibration description file (required)
- `-x, --hex <HEX>` - Path to the Intel HEX flash image (required)
- `-o, --output <OUTPUT>` - Output DCM file path (default: `generated.dcm`)

## DCM File Format

DCM (DAMOS Calibration Memory) files follow the ASAM standard format. Here's an example structure:

```text
* encoding="UTF-8"
* DAMOS format
* Created by CDM V7.2.17 Build 86
* Creation date: 2025/2/20 19:34:19
*
* Project: Example_Project
* Dataset: example.DCM

KONSERVIERUNG_FORMAT 2.0

* no memory layouts specified

FESTWERT VAR_0001
   LANGNAME "Control enable flag"
   EINHEIT_W "unitless"
   WERT 1.0000000000000000
END

GRUPPENKENNLINIE VAR_0002 9
   LANGNAME "Calibration lookup table"
   EINHEIT_X "percent"
   EINHEIT_W "mA"
*SSTX	VAR_0003
   ST/X   0.0000000000000000   12.5000000000000000   25.0000000000000000
   ST/X   37.5000000000000000   50.0000000000000000   62.5000000000000000
   WERT   320.0000000000000000   480.0000000000000000   640.0000000000000000
   WERT   800.0000000000000000   960.0000000000000000   1120.0000000000000000
END
```

### Block Structure

Each calibration parameter is defined as a block with:
- **Block header**: Block type and name
- **LANGNAME**: Description/label
- **EINHEIT_W/X/Y**: Units for values and axes
- **Values**: WERT (numeric) or TEXT (string) values
- **END**: Block terminator

## Architecture

### Module Structure

```
src/
├── main.rs          # CLI entry point
├── lib.rs           # Library core (DcmData, I/O operations)
├── block.rs         # Block enum (unified interface)
├── value.rs         # Value enum (WERT/TEXT)
├── diff.rs          # Diff functionality
├── gen.rs           # A2L+HEX to DCM generation
├── blocks/          # Block type implementations
│   ├── festwert.rs
│   ├── festwerteblock.rs
│   ├── gruppenkennlinie.rs
│   ├── stuetzstellenverteilung.rs
│   └── gruppenkennfeld.rs
└── attr/            # Attribute parsing
    ├── string_attr.rs
    ├── value_attr.rs
    └── attr_arbitor.rs
```

### Key Components

- **DcmData**: Main data structure holding all calibration blocks in an `IndexMap`
- **Block Enum**: Unified interface for all block types with common operations
- **Value Enum**: Represents either numeric (`WERT`) or text (`TEXT`) values
- **Handlebars Templates**: Used for formatting output DCM files

### Data Flow

1. **Parse**: `DcmData::new(path)` reads and parses DCM file into blocks
2. **Store**: Blocks stored in `IndexMap<String, Block>` (preserves order)
3. **Manipulate**: Operations like merge, update, filter modify the map
4. **Render**: `write_dcm_data()` uses templates to generate output

## Development

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_festwert

# Run tests with output visible
cargo test -- --nocapture
```

### Code Quality

```bash
# Run clippy (configured with .clippy.toml)
cargo clippy

# Format code
cargo fmt
```

### Project Configuration

- **Rust Version**: 1.94.0 or later
- **Clippy**: Configured with `too-many-arguments-threshold = 12`

### Key Dependencies

| Crate | Purpose |
|-------|---------|
| `indexmap` | Ordered map for consistent block ordering |
| `handlebars` | Template engine for DCM file generation |
| `clap` | CLI argument parsing with derive macros |
| `serde_json` | JSON serialization for diff output |
| `regex` | Pattern matching for filter command |
| `a2ldeser` | A2L+HEX calibration data extraction |
| `a2lfile` | A2L file parser |
| `colored` | Terminal color output |
| `rstest` | Parametrized testing framework |

## Examples

### Workflow: Update Calibration from New Data

```bash
# 1. Compare base with new data to see what changed
dcm_utils diff base.DCM new_data.DCM -o changes.json

# 2. Update base file with new values
dcm_utils update base.DCM new_data.DCM -o updated.DCM

# 3. Verify the update
dcm_utils diff base.DCM updated.DCM
```

### Workflow: Merge Multiple Calibration Files

```bash
# Merge multiple files, keeping base.DCM as foundation
dcm_utils merge base.DCM additions1.DCM additions2.DCM -o complete.DCM
```

### Workflow: Extract Specific Calibration Sets

```bash
# Extract all specific parameters
dcm_utils filter large_dataset.DCM --include "VAR.*" -o var_only.DCM

# Extract all parameters except temporary/test ones
dcm_utils filter dataset.DCM --exclude ".*Temp.*" ".*Test.*" -o clean.DCM
```

## Testing

The project includes comprehensive tests:

- **Unit Tests**: Block parsing, value handling, attribute parsing
- **Integration Tests**: Full file read/write cycles
- **Smoke Tests**: Parse all test DCM files to ensure no panics

Test data is located in `./test-dcms/` directory.

## License

[Specify your license here]

## Contributing

Contributions are welcome! Please ensure:

1. Code follows Rust conventions (`cargo fmt`, `cargo clippy`)
2. Tests pass (`cargo test`)
3. New features include appropriate tests
4. Documentation is updated as needed

## Acknowledgments

- ASAM e.V. for the DAMOS standard
- The Rust community for excellent tooling and libraries
