# GEN Command Design: A2L+HEX to DCM Generation

## Goal

Add a `gen` CLI subcommand that reads an A2L calibration description file and an Intel HEX flash image, extracts all calibration characteristic values, and writes them to a DCM file.

## Command Syntax

```bash
cargo run -- gen --a2l a.a2l --hex b.hex --output all_cali.DCM
```

- `--a2l` (required): Path to the A2L file
- `--hex` (required): Path to the Intel HEX flash image
- `--output` (default: `generated.dcm`): Output DCM file path

## Architecture

### Module Structure

- `src/gen.rs`: New module containing `extracted_to_dcm_blocks()` — converts a2ldeser `ExtractedObject` results into `Block` instances
- `src/main.rs`: Add `Gen` variant to the `Commands` enum with the CLI handler
- `src/lib.rs`: Add `pub mod gen;`

### Data Flow

```
CLI args (--a2l, --hex, --output)
  → a2lfile::load(OsString, None, false) + HexMemory::from_file()
  → Extractor::new(module, &hex_mem).extract_all()
  → ExtractionReport { successes, failures }
  → gen::extracted_to_dcm_blocks(successes, module)
  → IndexMap<String, Block>
  → DcmData::from_blocks(blocks)
  → render_to_file(&output)
```

### Dependencies

Add to `Cargo.toml`:
```toml
a2ldeser = { git = "https://github.com/huibing/a2ldeser" }
```

`a2lfile` is a transitive dependency of `a2ldeser` and will be available.

## Type Mapping: a2ldeser → DCM Block

| a2ldeser Type | DCM Block Type | Notes |
|---|---|---|
| `ExtractedValue` | `FESTWERT` | Single scalar. Numeric → from_f64, Verbal → from_string |
| `ExtractedValBlk` | `FESTWERTEBLOCK` | Array of values. See mixed-value handling below |
| `ExtractedCurve` | `GRUPPENKENNLINIE` + `STUETZSTELLENVERTEILUNG` | 1D lookup. Axis block generated first |
| `ExtractedMap` | `GRUPPENKENNFELD` + 2× `STUETZSTELLENVERTEILUNG` | 2D lookup. X and Y axis blocks generated first |
| `ExtractedAscii` | `FESTWERT` (TEXT) | String stored as TEXT value, EINHEIT_W empty |
| `ExtractedAxisPts` | `STUETZSTELLENVERTEILUNG` | Standalone axis distribution |

### Axis Naming Convention

CURVE/MAP axis distributions need names for `*SSTX`/`*SSTY` references:
- CURVE X-axis: `{curve_name}_X`
- MAP X-axis: `{map_name}_X`
- MAP Y-axis: `{map_name}_Y`

**Collision handling**: If a derived axis name (e.g., `MyCurve_X`) collides with a standalone `ExtractedAxisPts` of the same name, the standalone AXIS_PTS takes priority. A warning is printed to stderr. This is checked by attempting insertion into the IndexMap — if the key already exists, the derived axis block is skipped.

### PhysicalValue Handling

- `PhysicalValue::Numeric(f64)` → use the f64 value directly
- `PhysicalValue::Verbal(String)` in FESTWERT/FESTWERTEBLOCK → store as TEXT
- `PhysicalValue::Verbal(String)` in CURVE/MAP → the entire block is **skipped** with a warning. A partially-valid lookup table with NaN values is unreliable for downstream consumers. The block name and reason are printed to stderr.

**Mixed-value FESTWERTEBLOCK**: If any element in `ExtractedValBlk.values` is `Verbal`, the entire block is converted to TEXT. Numeric elements are rendered as their string representation (e.g., `3.14` → `"3.14"`). This preserves all data and is consistent with the DCM TEXT format.

### Metadata Population

DCM blocks require LANGNAME and EINHEIT fields. The `Extracted*` structs from a2ldeser contain `name` and `unit` fields but do **not** contain `long_identifier`. Metadata lookup strategy:

1. **LANGNAME**: Look up the characteristic's `long_identifier` by iterating `module.characteristic` and matching on name. If not found (e.g., standalone AXIS_PTS that is not a CHARACTERISTIC), fall back to the characteristic name itself as LANGNAME.
2. **EINHEIT_W**: Use the `unit` field from the `Extracted*` struct directly (already resolved by the extractor from COMPU_METHOD).
3. **EINHEIT_X / EINHEIT_Y**: Use the `x_unit` / `y_unit` fields from `ExtractedCurve` / `ExtractedMap` directly.
4. **EINHEIT_W for ASCII**: Empty string — ASCII characteristics have no meaningful unit.
5. **LANGNAME for AXIS_PTS**: Look up in `module.axis_pts` by name for the `long_identifier`. If not found, use the name itself.

The `extracted_to_dcm_blocks()` function takes both the `Vec<ExtractedObject>` successes and a reference to the A2L `Module` for metadata lookups.

### Block Insertion Order

Blocks are inserted into `IndexMap` grouped by type to match the template rendering sections:
1. FESTWERT (Value + Ascii)
2. FESTWERTEBLOCK
3. STUETZSTELLENVERTEILUNG (standalone + derived axis blocks)
4. GRUPPENKENNLINIE
5. GRUPPENKENNFELD

Within each type group, blocks maintain their original extraction order as returned by a2ldeser. The `extracted_to_dcm_blocks()` function partitions `ExtractedObject` instances by type first, then inserts in the specified group order.

## Error Handling

- **A2L/HEX load failure**: Return error immediately — nothing to generate
- **Individual characteristic extraction failure**: Skip silently, include in summary
- **Verbal value in CURVE/MAP**: Skip the entire block with a warning to stderr
- **Mixed Verbal/Numeric in ValBlk**: Convert entire block to TEXT format
- **Axis name collision**: Standalone AXIS_PTS wins, derived axis skipped with warning
- **Zero successful extractions**: Print warning, still write empty DCM file
- **I/O on output**: Follow existing convention — `render_to_file` panics on write failure (consistent with other commands)

The `ExtractionReport` from a2ldeser provides `successes: Vec<ExtractedObject>` and `failures: Vec<ExtractionFailure>`. The gen command prints a summary of failures to stderr before writing the DCM file.

## CLI Handler (main.rs)

```rust
Gen { a2l, hex, output } => {
    // a2lfile::load takes OsString, returns (A2lFile, Vec<ParseWarning>)
    let a2l_path = std::ffi::OsString::from(a2l.as_os_str());
    let (a2l_obj, _warnings) = a2lfile::load(a2l_path, None, false)
        .map_err(|e| anyhow::anyhow!("Failed to parse A2L file: {}", e))?;
    let module = &a2l_obj.project.module[0];
    let hex_mem = HexMemory::from_file(&hex)
        .map_err(|e| anyhow::anyhow!("Failed to parse HEX file: {}", e))?;
    let extractor = Extractor::new(module, &hex_mem);
    let report = extractor.extract_all();
    // Print failure summary to stderr
    for fail in &report.failures {
        eprintln!("SKIP: {} - {:?}", fail.name, fail.error);
    }
    eprintln!("Extracted {}/{} characteristics", report.successes.len(), report.total());
    let blocks = gen::extracted_to_dcm_blocks(report.successes, module);
    let dcm_data = DcmData::from_blocks(blocks);
    dcm_data.render_to_file(&output);
}
```

Note: `anyhow` may need to be added as a dependency if not already present, or the error handling can use `Box<dyn Error>` matching the existing pattern.

## Testing

- Unit tests in `gen.rs` for each type mapping using helper functions to construct `ExtractedObject` test instances (factory functions like `make_extracted_value(name, physical, unit)`)
- Integration test: create minimal A2L+HEX test fixture files in `./test-dcms/` (or a new `./test-a2l/` directory), run gen, parse the output DCM with `DcmData::new()`, verify blocks match expected values
- Test edge cases: empty A2L, Verbal values in CURVE (block skipped), mixed ValBlk (converted to TEXT), zero successful extractions, axis name collisions
- Test fixtures: small hand-crafted A2L file with one of each characteristic type + matching HEX file with known values at the specified addresses
