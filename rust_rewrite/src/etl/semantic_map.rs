use std::{fs::{self, File}, io::{Read, Write}, path::{Path, PathBuf}};

use crate::{data::{OsmMapData, SemanticMapElements}, errors::Result};
use crate::etl::parse_osm;

use super::Etl;

const ETL_NAME: &str = "semantic_map";
const OUTPUT_FILE_NAME: &str = "semantic_map.rkyv";

pub struct SemanticMapEtl {
}

impl SemanticMapEtl {
    fn output_path(dir: &Path) -> PathBuf {
        dir.join(OUTPUT_FILE_NAME)
    }

    pub fn new() -> SemanticMapEtl {
        SemanticMapEtl {}
    }
}

impl Etl for SemanticMapEtl {
    type Input = OsmMapData;
    type Output = SemanticMapElements;

    fn etl_name(&self) -> &str {
        ETL_NAME
    }

    fn is_cached(&self, dir: &std::path::Path) -> Result<bool> {
        Ok(Self::output_path(dir).exists())
    }

    fn clean(&self, dir: &std::path::Path) -> Result<()> {
        fs::remove_file(Self::output_path(dir))?;
        Ok(())
    }

    fn extract(&mut self, dir: &std::path::Path) -> Result<Self::Input> {
        let input_file_path = dir.join(parse_osm::OUTPUT_FILE_NAME);
        let mut input_file = File::open(input_file_path)?;

        let mut buf_vec: Vec<u8> = Vec::new();
        input_file.read_to_end(&mut buf_vec).expect("Could not read note cache.");

        let input: OsmMapData = unsafe {
            rkyv::from_bytes_unchecked(&buf_vec).expect("Could not deserialize node cache.")
        };

        Ok(input)
    }

    fn transform(&mut self, input: Self::Input) -> Result<Self::Output> {
        todo!()
    }

    fn load(&mut self, dir: &std::path::Path, output: Self::Output) -> Result<()> {
        let mut output_file = File::create(Self::output_path(dir))?;
        let bytes = rkyv::to_bytes::<_, 256>(&output).unwrap();
        output_file.write_all(&bytes)?;
        Ok(())
    }

}
