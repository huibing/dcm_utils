use chrono::Local;
use clap::{Parser, Subcommand};
use colored::Colorize;
use dcm_utils::{
    compute_multi_source_diff, dcm_diff_with_metadata, gen, merge_dcm_data, serve,
    update_dcm_data, validate_and_build_sources, CalSource, DcmData, DcmDiff,
};
use env_logger::Builder;
use std::io::Write;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "DCM Utils")]
#[command(about = "A tool to merge and update DCM files", long_about = None)]
#[command(version = "1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Merge multiple DCM files into one using the first file as the base
    ///
    /// If calibration data collides, the first file will be used as the base.
    /// If the first file has a variable that is not in the other files, it will be kept.
    /// If the other files have variables that are not in the first file, they will be added to the merged file.
    ///
    /// ## Examples
    ///
    /// Merge two DCM files:
    ///
    ///     dcm_utils merge base.DCM additions.DCM
    ///
    /// Merge multiple files with custom output name:
    ///
    ///     dcm_utils merge base.DCM part1.DCM part2.DCM part3.DCM -o complete.DCM
    Merge {
        dcms: Vec<PathBuf>,
        #[arg(short, long, default_value = "merged.dcm")]
        output: PathBuf,
    },
    /// Update the first DCM file with the data from the other DCM files
    ///
    /// If calibration variables does not exist in the first DCM file, they will be discarded.
    /// New variables from update files are not added; only existing variables are updated.
    ///
    /// ## Examples
    ///
    /// Update base file with new values from another file:
    ///
    ///     dcm_utils update base.DCM new_values.DCM
    ///
    /// Apply multiple update files sequentially:
    ///
    ///     dcm_utils update base.DCM updates1.DCM updates2.DCM -o final.DCM
    Update {
        dcms: Vec<PathBuf>,
        #[arg(short, long, default_value = "updated.dcm")]
        output: PathBuf,
    },
    /// Filter the DCM files by a given regex pattern
    ///
    /// Include only variables matching the given patterns, or exclude variables that match.
    /// Either --include or --exclude must be provided, but not both.
    ///
    /// ## Examples
    ///
    /// Include only variables starting with "VAR_":
    ///
    ///     dcm_utils filter input.DCM --include "VAR_.*"
    ///
    /// Include multiple patterns:
    ///
    ///     dcm_utils filter input.DCM --include "VAR_.*" "CFG_.*" -o subset.DCM
    ///
    /// Exclude temporary/test variables:
    ///
    ///     dcm_utils filter input.DCM --exclude ".*Temp.*" ".*Test.*" -o clean.DCM
    Filter {
        dcm: PathBuf,
        #[arg(short, long)]
        include: Option<Vec<String>>,
        #[arg(short, long)]
        exclude: Option<Vec<String>>,
        #[arg(short, long, default_value = "filtered.dcm")]
        output: PathBuf,
    },
    /// Compare calibration data from two sources (DCM files or A2L+HEX pairs)
    ///
    /// Generates a detailed comparison showing new, deleted, and changed variables.
    /// Results are printed to console and saved as JSON.
    ///
    /// ## Examples
    ///
    /// Compare two DCM files:
    ///
    ///     dcm_utils diff --dcm original.DCM --dcm modified.DCM
    ///
    /// Compare A2L+HEX against a DCM reference:
    ///
    ///     dcm_utils diff --dcm ref.DCM --a2l cal.a2l -x flash.hex
    ///
    /// Compare two A2L+HEX calibration sets:
    ///
    ///     dcm_utils diff --a2l v1.a2l -x v1.hex --a2l v2.a2l -x v2.hex
    ///
    /// Review the JSON output for detailed changes:
    ///
    ///     cat diff.json | jq '.[] | select(.Changed)'
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
        /// Serve diff results as a web page
        #[arg(long, default_value_t = false)]
        web: bool,
    },
    /// Compare variables from a base calibration source against multiple other sources
    ///
    /// Only variables present in the base source are compared. Variables not in the base
    /// are ignored. Each variable is compared across all sources using f32 byte-level comparison.
    ///
    /// ## Examples
    ///
    /// Compare a base DCM against two other DCM files:
    ///
    ///     dcm_utils diff-base --base base.DCM --other source1.DCM --other source2.DCM
    ///
    /// Compare an A2L+HEX base against DCM files:
    ///
    ///     dcm_utils diff-base --base --a2l cal.a2l -x flash.hex --other modified.DCM
    DiffBase {
        /// Base calibration source (DCM file)
        #[arg(long)]
        base_dcm: Option<PathBuf>,
        /// Base calibration source (A2L file, paired with --base-hex)
        #[arg(long)]
        base_a2l: Option<PathBuf>,
        /// Base Intel HEX flash image (paired with --base-a2l)
        #[arg(long = "base-hex")]
        base_hex: Option<PathBuf>,
        /// Other calibration sources (DCM files)
        #[arg(long)]
        other_dcm: Vec<PathBuf>,
        /// Other calibration sources (A2L files, paired with --other-hex)
        #[arg(long)]
        other_a2l: Vec<PathBuf>,
        /// Other Intel HEX flash images (paired with --other-a2l)
        #[arg(long = "other-hex")]
        other_hex: Vec<PathBuf>,
        /// Output JSON file
        #[arg(short, long, default_value = "diff-base.json")]
        output: PathBuf,
        /// Serve diff results as a web page
        #[arg(long, default_value_t = false)]
        web: bool,
    },
    /// Generate DCM file from A2L and HEX calibration files
    ///
    /// Extracts all calibration characteristics from an A2L file and HEX flash image,
    /// converting them to DCM format. Failed extractions are skipped with warnings.
    ///
    /// ## Examples
    ///
    /// Basic usage:
    ///
    ///     dcm_utils gen --a2l calibration.a2l -x flash.hex
    ///
    /// Custom output file:
    ///
    ///     dcm_utils gen --a2l calibration.a2l -x flash.hex --output all_cali.DCM
    Gen {
        /// Path to the A2L calibration description file
        #[arg(short, long)]
        a2l: PathBuf,
        /// Path to the Intel HEX flash image
        #[arg(short = 'x', long)]
        hex: PathBuf,
        /// Output DCM file path
        #[arg(short, long, default_value = "generated.dcm")]
        output: PathBuf,
    },
}

/// Build a single CalSource from CLI arguments. Exactly one of dcm or (a2l, hex) must be provided.
fn build_single_source(
    label: &str,
    dcm: &Option<PathBuf>,
    a2l: &Option<PathBuf>,
    hex: &Option<PathBuf>,
) -> Result<CalSource, String> {
    match (dcm, a2l, hex) {
        (Some(p), None, None) => Ok(CalSource::Dcm(p.clone())),
        (None, Some(a), Some(h)) => Ok(CalSource::A2lHex {
            a2l: a.clone(),
            hex: h.clone(),
        }),
        (None, Some(_), None) => Err(format!(
            "--{}-a2l requires --{}-hex",
            label, label
        )),
        (None, None, Some(_)) => Err(format!(
            "--{}-hex requires --{}-a2l",
            label, label
        )),
        (Some(_), Some(_), _) | (Some(_), _, Some(_)) => Err(format!(
            "Cannot provide both --{}-dcm and --{}-a2l/--{}-hex for the same source",
            label, label, label
        )),
        (None, None, None) => Err(format!(
            "Must provide either --{}-dcm or --{}-a2l with --{}-hex",
            label, label, label
        )),
    }
}

fn main() {
    let mut logger = Builder::new();
    logger.format(|buf, record| {
        let now = Local::now();
        let timestamp = now.format("%Y-%m-%d %H:%M:%S").to_string();
        writeln!(
            buf,
            "[{}] [{}] - {}",
            timestamp,
            record.level(),
            record.args()
        )
    });
    logger
        .filter_level(log::LevelFilter::Info)
        .try_init()
        .unwrap();
    let cli = Cli::parse();
    match cli.command {
        Commands::Merge { dcms, output } => {
            let main = dcms.first().expect("At least one DCM file is required");
            let others = &dcms[1..];
            let mut main_dcm = DcmData::new(main);
            let other_dcms: Vec<DcmData> = others.iter().map(|p| DcmData::new(p)).collect();
            println!(
                "Merging {} DCM files into {}",
                dcms.len().to_string().on_white().red(),
                output.to_str().unwrap().on_white().green()
            );
            merge_dcm_data(&mut main_dcm, other_dcms);
            main_dcm.render_to_file(&output);
        }
        Commands::Update { dcms, output } => {
            let mut dcm = DcmData::new(&dcms[0]);
            let other_dcms: Vec<DcmData> = dcms.iter().skip(1).map(|p| DcmData::new(p)).collect();
            update_dcm_data(&mut dcm, other_dcms);
            dcm.render_to_file(&output);
        }
        Commands::Filter {
            dcm,
            include,
            exclude,
            output,
        } => {
            let mut dcm = DcmData::new(&dcm);
            //dcm.filter_by_regex(&pattern);
            if let Some(include_pats) = include {
                dcm.filter_include(&include_pats);
            } else if let Some(exclude_pats) = exclude {
                dcm.filter_exclude(&exclude_pats);
            } else {
                panic!("Either include or exclude patterns must be provided");
            }
            dcm.render_to_file(&output);
        }
        Commands::Diff {
            dcm,
            a2l,
            hex,
            output,
            web,
        } => {
            let (left_src, right_src) = validate_and_build_sources(&dcm, &a2l, &hex)
                .unwrap_or_else(|e| {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                });

            let left_data = left_src.load().unwrap_or_else(|e| {
                eprintln!("Error loading {}: {}", left_src.label(), e);
                std::process::exit(1);
            });
            let right_data = right_src.load().unwrap_or_else(|e| {
                eprintln!("Error loading {}: {}", right_src.label(), e);
                std::process::exit(1);
            });

            let result =
                dcm_diff_with_metadata(&left_data, &right_data, &left_src, &right_src);

            // Print summary
            println!("{}", "=== Calibration Diff Results ===".bold());
            println!("Left:  {}", result.metadata.left_label.cyan());
            println!("Right: {}", result.metadata.right_label.cyan());
            println!("Timestamp: {}\n", result.metadata.timestamp.dimmed());

            println!(
                "New blocks: {}",
                result.summary.new_count.to_string().green()
            );
            println!(
                "Deleted blocks: {}",
                result.summary.deleted_count.to_string().red()
            );
            println!(
                "Changed blocks: {}",
                result.summary.changed_count.to_string().yellow()
            );
            println!(
                "Total differences: {}\n",
                result.summary.total.to_string().bold()
            );

            // Print detailed differences to terminal
            if !result.differences.is_empty() {
                println!("{}", "=== Detailed Differences ===".bold());
                for diff in &result.differences {
                    match diff {
                        DcmDiff::New { name, description, .. } => {
                            println!(
                                "{} {}: {}",
                                "[NEW]".green().bold(),
                                name.green(),
                                description.as_ref().unwrap_or(&"".to_string())
                            );
                        }
                        DcmDiff::Deleted { name, description, .. } => {
                            println!(
                                "{} {}: {}",
                                "[DEL]".red().bold(),
                                name.red(),
                                description.as_ref().unwrap_or(&"".to_string())
                            );
                        }
                        DcmDiff::Changed {
                            name, description, ..
                        } => {
                            println!(
                                "{} {}: {}",
                                "[CHG]".yellow().bold(),
                                name.yellow(),
                                description
                                    .as_ref()
                                    .unwrap_or(&"values changed".to_string())
                            );
                        }
                        DcmDiff::ChangedMap {
                            name, description, ..
                        } => {
                            println!(
                                "{} {}: {}",
                                "[CHG]".yellow().bold(),
                                name.yellow(),
                                description.as_ref().unwrap_or(&"map changed".to_string())
                            );
                        }
                    }
                }
                println!();
            }

            // Write diff result to JSON file
            let json = serde_json::to_string_pretty(&result).unwrap();
            std::fs::write(&output, json).expect("Failed to write diff output");
            println!(
                "Diff details written to: {}",
                output.display().to_string().blue()
            );

            if web {
                println!();
                serve::start(result).unwrap_or_else(|e| {
                    eprintln!("Server error: {}", e);
                    std::process::exit(1);
                });
            }
        }
        Commands::DiffBase {
            base_dcm,
            base_a2l,
            base_hex,
            other_dcm,
            other_a2l,
            other_hex,
            output,
            web,
        } => {
            // Build base source
            let base_src = build_single_source("base", &base_dcm, &base_a2l, &base_hex)
                .unwrap_or_else(|e| {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                });

            // Build other sources from --other-dcm and --other-a2l/--other-hex pairs
            let mut sources = vec![base_src];
            for path in &other_dcm {
                sources.push(CalSource::Dcm(path.clone()));
            }
            // Pair --other-a2l with --other-hex by index
            if other_a2l.len() != other_hex.len() {
                eprintln!(
                    "Error: Mismatched --other-a2l ({} provided) and --other-hex ({} provided)",
                    other_a2l.len(),
                    other_hex.len()
                );
                std::process::exit(1);
            }
            for (a, h) in other_a2l.iter().zip(other_hex.iter()) {
                sources.push(CalSource::A2lHex {
                    a2l: a.clone(),
                    hex: h.clone(),
                });
            }

            if sources.len() < 2 {
                eprintln!("Error: At least 2 sources (base + at least 1 other) are required");
                std::process::exit(1);
            }

            let result = compute_multi_source_diff(&sources).unwrap_or_else(|e| {
                eprintln!("Error computing diff: {}", e);
                std::process::exit(1);
            });

            // Print summary
            println!("{}", "=== Multi-Source Diff Results ===".bold());
            println!(
                "Base: {}",
                result.metadata.sources[0].cyan()
            );
            for (i, src) in result.metadata.sources[1..].iter().enumerate() {
                println!("Source {}: {}", i + 1, src.cyan());
            }
            println!();
            println!(
                "Total variables in base: {}",
                result.total_variables.to_string().bold()
            );
            println!(
                "Variables with differences: {}",
                result.variables_with_diffs.to_string().yellow().bold()
            );
            println!();

            // Print all base variables with comparison status
            println!("{}", "=== All Base Variables ===".bold());
            for diff in &result.differences {
                let missing_sources: Vec<String> = diff
                    .source_values
                    .iter()
                    .enumerate()
                    .filter(|(_, sv)| !sv.present)
                    .map(|(i, _)| {
                        if i == 0 { "base".to_string() } else { format!("source {}", i) }
                    })
                    .collect();

                let diff_sources: Vec<String> = diff
                    .source_values
                    .iter()
                    .enumerate()
                    .skip(1)
                    .filter(|(_, sv)| {
                        sv.present
                            && !sv.value.as_ref().is_some_and(|v| {
                                v.f32_bytes_eq(&diff.source_values[0].value.as_ref().unwrap())
                            })
                    })
                    .map(|(i, _)| format!("source {}", i))
                    .collect();

                let (prefix, desc) = if !diff.has_diff {
                    ("[OK]".green().bold(), "all match".to_string())
                } else if missing_sources.iter().any(|s| s == "base") {
                    ("[WARN]".yellow().bold(), "missing in [base]".to_string())
                } else {
                    let mut parts = Vec::new();
                    if !missing_sources.is_empty() {
                        parts.push(format!("missing in [{}]", missing_sources.join(", ")));
                    }
                    if !diff_sources.is_empty() {
                        parts.push(format!("differs in [{}]", diff_sources.join(", ")));
                    }
                    ("[CHG]".yellow().bold(), parts.join("; "))
                };

                println!(
                    "{} {} ({}){}",
                    prefix,
                    diff.name.yellow(),
                    diff.block_type,
                    if desc.is_empty() { String::new() } else { format!(": {}", desc) }
                );
            }
            println!();

            // Write JSON output
            let json = serde_json::to_string_pretty(&result).unwrap();
            std::fs::write(&output, json).expect("Failed to write diff output");
            println!(
                "Diff details written to: {}",
                output.display().to_string().blue()
            );

            if web {
                println!();
                serve::start_multi_source(result).unwrap_or_else(|e| {
                    eprintln!("Server error: {}", e);
                    std::process::exit(1);
                });
            }
        }
        Commands::Gen { a2l, hex, output } => {
            let dcm_data = gen::gen_dcm_data(&a2l, &hex).expect("Failed to generate DCM data");
            dcm_data.render_to_file(&output);
            println!(
                "DCM file written to: {}",
                output.display().to_string().blue()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use dcm_utils::DcmData;
    use env_logger::Builder;
    use ihex::Record;
    use log::{info, LevelFilter, SetLoggerError};
    use rstest::*;
    use std::fs::read_dir;
    use std::io::Read;
    use std::path::Path;

    #[fixture]
    #[once]
    fn tester_logger() -> Result<(), SetLoggerError> {
        let mut logger = Builder::new();
        logger
            .filter_level(LevelFilter::Info)
            .is_test(true)
            .try_init()
    }

    #[rstest]
    fn dcm_file_smoke_test() {
        let entries = read_dir("./test-dcms").unwrap();
        for entry in entries {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_file() && path.extension().unwrap() == "DCM" {
                let _ = DcmData::new(&path);
            }
        }
    }

    #[rstest]
    fn dcm_file_smoke_test2(tester_logger: &Result<(), SetLoggerError>) {
        let _ = tester_logger.as_ref().unwrap();
        let entries = read_dir("./test-dcms").unwrap();
        for entry in entries {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_file() && path.extension().unwrap() == "DCM" {
                info!("Start to parse File: {}", path.display());
                let d = DcmData::new(&path);
                assert_ne!(d.get_all_variable_names().len(), 0);
                info!(
                    "File: {} has {} variables",
                    path.display(),
                    d.get_all_variable_names().len()
                );
            }
        }
    }

    #[rstest]
    fn dcm_parse_test() {
        // Use sanitized test file
        let path = Path::new("./test-dcms/test_sample_677.DCM");
        let d = DcmData::new(&path);
        // Verify file has variables
        assert!(d.get_all_variable_names().len() > 0);
        // Test accessing a constant (VAR_0019 is CDCAct_DmprIMax_C)
        let constant = d.blocks.get("VAR_0019").unwrap();
        assert_relative_eq!(
            constant.get_values().try_into_f64().unwrap()[0],
            1800f64,
            epsilon = 1.0
        );
        // Test accessing a table (VAR_0020 is CDCAct_DmprIMaxFrnt_T)
        let table = d.blocks.get("VAR_0020").unwrap();
        assert_eq!(*table.get_values().try_into_f64().unwrap(), vec![1600.0; 8]);
    }

    #[rstest]
    fn test_ihex() {
        use std::io::{BufRead, BufReader};
        let path = "./test-dcms/1.hex";
        let file = std::fs::File::open(path).unwrap();
        let buf = BufReader::new(file);
        for line in buf.lines() {
            let line = line.unwrap();
            println!("Line: {}", line);
            let _ = Record::from_record_string(line.as_str()).unwrap();
        }
    }

    #[rstest]
    fn test_ihex_whole() {
        use ihex::Reader;
        let path = "./test-dcms/1.hex";
        let mut file = std::fs::File::open(path).unwrap();
        let mut s = String::new();
        file.read_to_string(&mut s).unwrap();
        s = s.replace("\r\n", "\n");
        let mut reader = Reader::new(s.as_str());
        let target_addr = 0x4000u16;
        let item = reader.find(|record| {
            if let Ok(rec) = record {
                if let Record::Data { offset, value } = rec {
                    if offset <= &target_addr && offset + value.len() as u16 > target_addr {
                        return true; // Stop skipping
                    }
                }
            }
            false
        });
        if let Some(Ok(record)) = item {
            if let Record::Data { offset, value } = record {
                println!(
                    "Record at address {:#x}: {:?} \n len: {}",
                    offset,
                    value,
                    value.len()
                );
            }
        } else {
            println!("No record found at address {}", target_addr);
        }
    }

    #[rstest]
    fn test_gen_basic() {
        let a2l_path = std::path::Path::new("./test-dcms/simple_test.a2l");
        let hex_path = std::path::Path::new("./test-dcms/simple_test.hex");
        let dcm_data =
            dcm_utils::gen::gen_dcm_data(a2l_path, hex_path).expect("gen_dcm_data should succeed");

        // Check VALUE -> FESTWERT
        assert!(dcm_data.contains_block("test_scalar"));
        let block = dcm_data.blocks.get("test_scalar").unwrap();
        let values = block.get_values().try_into_f64().unwrap();
        assert!((values[0] - 42.0).abs() < 0.001);

        // Check CURVE -> GRUPPENKENNLINIE
        assert!(dcm_data.contains_block("test_curve"));

        // Check VAL_BLK -> FESTWERTEBLOCK
        assert!(dcm_data.contains_block("test_valblk"));

        // Check that derived axis block was created for test_curve
        assert!(dcm_data.contains_block("test_curve_X"));
    }

    #[rstest]
    fn test_ihex_target() {
        use ihex::Reader;
        let path = "./test-dcms/1.hex";
        let mut file = std::fs::File::open(path).unwrap();
        let mut s = String::new();
        file.read_to_string(&mut s).unwrap();
        s = s.replace("\r\n", "\n");
        let mut reader = Reader::new(s.as_str());
        let target_addr = 0x80064020u32;
        let upper_addr = (target_addr >> 16) as u16;
        let lower_addr = (target_addr & 0xFFFF) as u16;
        let item = reader.find(|record| {
            if let Ok(rec) = record {
                if let Record::ExtendedLinearAddress(addr) = rec {
                    if addr == &upper_addr {
                        return true; // Stop skipping
                    }
                }
            }
            false
        });
        if let Some(Ok(_)) = item {
            let data = reader.find(|record| {
                if let Ok(rec) = record {
                    if let Record::Data { offset, value } = rec {
                        if offset <= &lower_addr && offset + value.len() as u16 > lower_addr {
                            return true; // Stop skipping
                        }
                    }
                }
                false
            });
            if let Some(Ok(record)) = data {
                if let Record::Data { offset, value } = record {
                    println!(
                        "Record at address {:#x}: {:?} \n len: {}",
                        offset,
                        value,
                        value.len()
                    );
                }
            } else {
                println!("No data record found at address {}", target_addr);
            }
        }
    }
}
