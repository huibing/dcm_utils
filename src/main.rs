fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod tests {
    use rstest::*;
    use dcm_parse::DcmData;
    use std::fs::read_dir;
    use log::{info, LevelFilter, SetLoggerError};
    use std::path::Path;
    use approx::assert_relative_eq;
    use env_logger::Builder;

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
        let path = Path::new("./test-dcms/NT3_ALPS_Blanc-RWDCOIL_Zone-Lite_XM_BL0100_20250220_LB_2.DCM");
        let d = DcmData::new(&path);
        assert_eq!(d.get_all_variable_names().len(), 660);
        let constant = d.blocks.get("IVE_WhlZSigPosnXFrntLft_C").unwrap();
        assert_relative_eq!(constant.get_values().try_into_f64().unwrap()[0], 1820.8, epsilon = 0.1);
        let constant = d.blocks.get("CDCAct_DmprIMax_C").unwrap();
        assert_relative_eq!(constant.get_values().try_into_f64().unwrap()[0], 1800f64, epsilon = 1.0);
        let table = d.blocks.get("CDCAct_DmprIMaxFrnt_T").unwrap();
        assert_eq!(*table.get_values().try_into_f64().unwrap(), vec![1600.0; 8]);
        let map = d.blocks.get("CDCBlnd_Sel_M").unwrap();
        assert_eq!(map.get_values().try_into_f64().unwrap().len(), 24);
        assert_eq!(*map.get_values().try_into_f64().unwrap(), vec![1.0; 24]);
        let map1 = d.blocks.get("CDCBlnd_EOTFrntLim_M").unwrap();
        assert_eq!(map1.get_values().try_into_f64().unwrap().len(), 32);
        assert_eq!(*map1.get_values().try_into_f64().unwrap(), vec![10.0,60.0,60.0,50.0,0.0,0.0,0.0,0.0,10.0,60.0,60.0,50.0,0.0,0.0,0.0,0.0,10.0,60.0,60.0,50.0,0.0,0.0,0.0,0.0,10.0,60.0,60.0,50.0,0.0,0.0,0.0,0.0]);
    }
}
