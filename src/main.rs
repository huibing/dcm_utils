fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod tests {
    use rstest::*;
    use dcm_parse::DcmData;
    use std::fs::read_dir;
    use log::info;

    #[fixture]
    fn dcm_data() {
        let _ = env_logger::builder().is_test(true).try_init();
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
    fn dcm_file_smoke_test2(dcm_data: ()) {
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
}
