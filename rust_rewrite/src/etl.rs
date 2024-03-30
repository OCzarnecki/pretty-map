pub mod parse_osm;

use std::{fs, path::{Path, PathBuf}};
use log::info;

use crate::errors::Result;


pub trait ETL {
    type Input;
    type Output;

    fn etl_name(&self) -> &str;
    fn output_file_name(&self) -> &str;

    fn extract(&self) -> Result<Self::Input>;
    fn transform(&self, input: Self::Input) -> Result<Self::Output>;
    fn load(&self, output_file: fs::File, output: Self::Output) -> Result<()>;

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

    fn process(&self, dir: &Path) -> Result<()> {
        info!(etl_name = self.etl_name(); "Starting ETL process");
        if self.is_cached(dir)? {
            info!(etl_name = self.etl_name(); "Using cached value");
        } else {
            let output_file = fs::File::create(self.output_path(dir))?;

            info!(etl_name = self.etl_name(); "Extracting");
            let input = self.extract()?;

            info!(etl_name = self.etl_name(); "Transforming");
            let output = self.transform(input)?;

            info!(etl_name = self.etl_name(); "Loading");
            self.load(output_file, output)?;
        }
        info!(etl_name = self.etl_name(); "Process finished");
        Ok(())
    }
}


