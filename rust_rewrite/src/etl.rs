pub mod parse_osm;

use std::{fs, path::{Path, PathBuf}};
use log::{info, error};

use crate::errors::Result;


pub trait Etl {
    type Input;
    type Output;

    fn etl_name(&self) -> &str;
    fn output_file_name(&self) -> &str;

    fn extract(&mut self) -> Result<Self::Input>;
    fn transform(&mut self, input: Self::Input) -> Result<Self::Output>;
    fn load(&mut self, output_file: fs::File, output: Self::Output) -> Result<()>;

    fn output_path(&self, dir: &Path) -> PathBuf {
        dir.join(self.output_file_name())
    }

    fn is_cached(&self, dir: &Path) -> Result<bool> {
        Ok(self.output_path(dir).try_exists()?)
    }

    fn clean(&self, dir: &Path) -> Result<()> {
        if self.is_cached(dir)? {
            Ok(fs::remove_file(self.output_path(dir))?)
        } else {
            Ok(())
        }
    }

    fn process(&mut self, dir: &Path) -> Result<()> {
        info!(etl_name = self.etl_name(); "Starting ETL process");
        if self.is_cached(dir)? {
            info!(etl_name = self.etl_name(); "Using cached value");
        } else {
            let output_file = fs::File::create(self.output_path(dir))?;

            info!(etl_name = self.etl_name(); "Extracting");
            let input = match self.extract() {
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
            match self.load(output_file, output) {
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


