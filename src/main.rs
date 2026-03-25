use dcm_utils::{
    DcmData,
    DcmDiff,
    dcm_diff,
    merge_dcm_data,
    update_dcm_data,
};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use env_logger::Builder;
use chrono::Local;
use std::io::Write;
use colored::Colorize;

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
    /// Compare two DCM files and show differences
    ///
    /// Generates a detailed comparison showing new, deleted, and changed variables.
    /// Results are printed to console and saved as JSON.
    ///
    /// ## Examples
    ///
    /// Compare two files with default output:
    ///
    ///     dcm_utils diff original.DCM modified.DCM
    ///
    /// Specify custom output file:
    ///
    ///     dcm_utils diff base.DCM new_version.DCM -o changes.json
    ///
    /// Review the JSON output for detailed changes:
    ///
    ///     cat diff.json | jq '.[] | select(.Changed)'
    Diff {
        /// Original/base DCM file
        original: PathBuf,
        /// Modified DCM file to compare against
        modified: PathBuf,
        /// Output JSON file for diff results
        #[arg(short, long, default_value = "diff.json")]
        output: PathBuf,
    },
}


fn main() {
    let mut logger = Builder::new();
    logger.format( |buf, record| {
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
    logger.filter_level(log::LevelFilter::Info).try_init().unwrap();
    let cli = Cli::parse();
    match cli.command {
        Commands::Merge { dcms, output } => {
            let main = dcms.first().expect("At least one DCM file is required");
            let others = &dcms[1..];
            let mut main_dcm = DcmData::new(main);
            let other_dcms: Vec<DcmData> = others.iter().map(|p| DcmData::new(p)).collect();
            println!("Merging {} DCM files into {}", dcms.len().to_string().on_white().red(), output.to_str().unwrap().on_white().green());
            merge_dcm_data(&mut main_dcm, other_dcms);
            main_dcm.render_to_file(&output);
        },
        Commands::Update { dcms, output } => {
            let mut dcm = DcmData::new(&dcms[0]);
            let other_dcms: Vec<DcmData> = dcms.iter().skip(1).map(|p| DcmData::new(p)).collect();
            update_dcm_data(&mut dcm, other_dcms);
            dcm.render_to_file(&output);
        },
        Commands::Filter { dcm, include, exclude, output } => {
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
        },
        Commands::Diff { original, modified, output } => {
            let original_dcm = DcmData::new(&original);
            let modified_dcm = DcmData::new(&modified);

            let diff = dcm_diff(&original_dcm, &modified_dcm);

            // Print summary
            let new_count = diff.iter().filter(|d| matches!(d, DcmDiff::New { .. })).count();
            let deleted_count = diff.iter().filter(|d| matches!(d, DcmDiff::Deleted { .. })).count();
            let changed_count = diff.iter().filter(|d| matches!(d, DcmDiff::Changed { .. } | DcmDiff::ChangedMap { .. })).count();

            println!("{}", "=== DCM Diff Results ===".bold());
            println!("New blocks: {}", new_count.to_string().green());
            println!("Deleted blocks: {}", deleted_count.to_string().red());
            println!("Changed blocks: {}", changed_count.to_string().yellow());
            println!("Total differences: {}", diff.len().to_string().bold());

            // Write diff to JSON file
            let json = serde_json::to_string_pretty(&diff).unwrap();
            std::fs::write(&output, json).expect("Failed to write diff output");
            println!("\nDiff details written to: {}", output.display().to_string().blue());
        },
    }
}

#[cfg(test)]
mod tests {
    use rstest::*;
    use dcm_utils::DcmData;
    use std::fs::read_dir;
    use log::{info, LevelFilter, SetLoggerError};
    use std::path::Path;
    use approx::assert_relative_eq;
    use env_logger::Builder;
    use ihex::Record;
    use std::io::Read;

    #[fixture]
    #[once]
    fn tester_logger() -> Result<(), SetLoggerError> {
        let mut logger = Builder::new();
        logger.filter_level(LevelFilter::Info).is_test(true).try_init()
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
    fn dcm_file_smoke_test2(tester_logger: &Result<(), SetLoggerError> ) {
        let _ = tester_logger.as_ref().unwrap();
        let entries = read_dir("./test-dcms").unwrap();
        for entry in entries {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_file() && path.extension().unwrap() == "DCM" {
                info!("Start to parse File: {}", path.display());
                let d = DcmData::new(&path);
                assert_ne!(d.get_all_variable_names().len(), 0);
                info!("File: {} has {} variables", path.display(), d.get_all_variable_names().len());
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
        assert_relative_eq!(constant.get_values().try_into_f64().unwrap()[0], 1800f64, epsilon = 1.0);
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
                println!("Record at address {:#x}: {:?} \n len: {}", offset, value, value.len());
            }
        } else {
            println!("No record found at address {}", target_addr);
        }
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
                if let Record::ExtendedLinearAddress (addr) = rec {
                    if addr == &upper_addr {
                        return true; // Stop skipping
                    }
                } 
            }
        false});
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
                    println!("Record at address {:#x}: {:?} \n len: {}", offset, value, value.len());
                }
            } else {
                println!("No data record found at address {}", target_addr);
            }
        }
    }
}
