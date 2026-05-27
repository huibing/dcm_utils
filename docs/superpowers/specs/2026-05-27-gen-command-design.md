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
  → a2lfile::load() + HexMemory::from_file()
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
| `ExtractedValBlk` | `FESTWERTEBLOCK` | Array of values. Numeric/Verbal dispatch |
| `ExtractedCurve` | `GRUPPENKENNLINIE` + `STUETZSTELLENVERTEILUNG` | 1D lookup. Axis block generated first |
| `ExtractedMap` | `GRUPPENKENNFELD` + 2× `STUETZSTELLENVERTEILUNG` | 2D lookup. X and Y axis blocks generated first |
| `ExtractedAscii` | `FESTWERT` (TEXT) | String stored as TEXT value |
| `ExtractedAxisPts` | `STUETZSTELLENVERTEILUNG` | Standalone axis distribution |

### Axis Naming Convention

CURVE/MAP axis distributions need names for `*SSTX`/`*SSTY` references:
- CURVE X-axis: `{curve_name}_X`
- MAP X-axis: `{map_name}_X`
- MAP Y-axis: `{map_name}_Y`

### PhysicalValue Handling

- `PhysicalValue::Numeric(f64)` → use the f64 value directly
- `PhysicalValue::Verbal(String)` → store as TEXT for FESTWERT/FESTWERTEBLOCK; for CURVE/MAP values, convert to `f64::NAN` and print a warning (verbal values in lookup tables are not representable in DCM format)

### Metadata Population

DCM blocks require LANGNAME and EINHEIT fields. These are populated from A2L metadata:
- `LANGNAME`: from the A2L characteristic's `long_identifier`
- `EINHEIT_W`: from the COMPU_METHOD `unit` attached to the characteristic
- `EINHEIT_X` / `EINHEIT_Y`: from the axis COMPU_METHOD `unit`

The `extracted_to_dcm_blocks()` function takes both the `Vec<ExtractedObject>` successes and a reference to the A2L `Module` so it can look up `long_identifier` and other metadata by characteristic name.

### Block Insertion Order

Blocks are inserted into `IndexMap` in this order to match the template rendering sections:
1. FESTWERT (Value + Ascii)
2. FESTWERTEBLOCK
3. STUETZSTELLENVERTEILUNG (standalone + derived axis blocks)
4. GRUPPENKENNLINIE
5. GRUPPENKENNFELD

## Error Handling

- **A2L/HEX load failure**: Return error immediately — nothing to generate
- **Individual characteristic extraction failure**: Skip silently, include in summary
- **Verbal value in CURVE/MAP**: Warning to stderr, use `f64::NAN`
- **Zero successful extractions**: Print warning, still write empty DCM file

The `ExtractionReport` from a2ldeser provides `successes: Vec<ExtractedObject>` and `failures: Vec<ExtractionFailure>`. The gen command prints a summary of failures to stderr before writing the DCM file.

## CLI Handler (main.rs)

```rust
Gen { a2l, hex, output } => {
    let (a2l_obj, _) = a2lfile::load(&a2l, None, false)?;
    let module = &a2l_obj.project.module[0];
    let hex_mem = HexMemory::from_file(&hex)?;
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

## Testing

- Unit tests in `gen.rs` for each type mapping (mock ExtractedObject → Block)
- Integration test: load a small A2L + HEX pair, run gen, parse the output DCM, verify blocks match
- Test edge cases: empty A2L, Verbal values, zero successful extractions
