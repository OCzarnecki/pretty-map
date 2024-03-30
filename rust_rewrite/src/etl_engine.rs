use std::{fs, io, path::{Path, PathBuf}};

#[derive(Debug)] 
pub struct Error {
    message: String,
}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Error {
            message: value.to_string()
        }
    }
}

trait ETL {
    type Input;
    type Output;

    fn input_file_name(&self) -> &str;
    fn output_file_name(&self) -> &str;

    fn extract(&self, input_file: fs::File) -> Result<Self::Input, Error>;
    fn transform(&self, input: Self::Input) -> Result<Self::Output, Error>;
    fn load(&self, output_file: fs::File, output: Self::Output) -> Result<(), Error>;

    fn input_path(&self, dir: &Path) -> PathBuf {
        dir.join(self.input_file_name())
    }
    
    fn output_path(&self, dir: &Path) -> PathBuf {
        dir.join(self.output_file_name())
    }

    fn is_cached(&self, dir: &Path) -> Result<bool, Error> {
        Ok(self.output_path(dir).try_exists()?)
    }

    fn clean(&self, dir: &Path) -> Result<(), Error> {
        if self.is_cached(dir)? {
            Ok(fs::remove_file(self.output_path(dir))?)
        } else {
            Ok(())
        }
    }

    fn process(&self, dir: &Path) -> Result<(), Error> {
        if self.is_cached(dir)? {
            return Ok(())
        }

        let mut input_file = fs::File::open(self.input_path(dir))?;
        let mut output_file = fs::File::create(self.output_path(dir))?;

        let input = self.extract(input_file)?;
        let output = self.transform(input)?;
        Ok(self.load(output_file, output)?)
    }
}


