pub mod parse_osm;
pub mod semantic_map;

use std::{fs, path::{Path, PathBuf}};
use log::{info, error};

use crate::errors::Result;


pub trait Etl {
    type Input;
    type Output;

    fn etl_name(&self) -> &str;

    fn is_cached(&self, dir: &Path) -> Result<bool>;
    fn clean(&self, dir: &Path) -> Result<()>;

    fn extract(&mut self, dir: &Path) -> Result<Self::Input>;
    fn transform(&mut self, input: Self::Input) -> Result<Self::Output>;
    fn load(&mut self, dir: &Path, output: Self::Output) -> Result<()>;

    fn process(&mut self, dir: &Path) -> Result<()> {
        info!(etl_name = self.etl_name(); "Starting ETL process");
        if self.is_cached(dir)? {
            info!(etl_name = self.etl_name(); "Using cached value");
        } else {
            info!(etl_name = self.etl_name(); "Extracting");
            let input = match self.extract(dir) {
                Ok(input) => Ok(input),
                Err(err) => {
                    error!(etl_name = self.etl_name(), err = err.message; "Extraction failed with error");
                    Err(err)
                },
            }?;
            
            info!(etl_name = self.etl_name(); "Transforming");
            let output = match self.transform(input) {
                Ok(output) => Ok(output),
                Err(err) => {
                    error!(etl_name = self.etl_name(), err = err.message; "Transformation failed with error");
                    Err(err)
                },
            }?;

            info!(etl_name = self.etl_name(); "Loading");
            match self.load(dir, output) {
                Ok(_) => Ok(()),
                Err(err) => {
                    error!(etl_name = self.etl_name(), err = err.message; "Loading failed with error");
                    Err(err)
                },
            }?;
        }
        info!(etl_name = self.etl_name(); "Process finished");
        Ok(())
    }
}


