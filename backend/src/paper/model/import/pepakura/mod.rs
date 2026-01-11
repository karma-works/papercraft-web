mod data;
mod importer;

pub use importer::PepakuraImporter;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use crate::paper::import::import_model_file;

    #[test]
    fn test_load_sports_car() {
        let path = Path::new("../proprietary/sample_craft_works/pepakura_sports_car.pdo");
        if path.exists() {
            let res = import_model_file(path);
            assert!(res.is_ok(), "Failed to load sports car: {:?}", res.err());
        }
    }
}
