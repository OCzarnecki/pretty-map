use std::io;

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

impl From<quick_xml::Error> for Error {
    fn from(value: quick_xml::Error) -> Self {
        Error {
            message: value.to_string()
        }
    }
}

impl From<&str> for Error {
    fn from(value: &str) -> Self {
        Error {
            message: value.to_string()
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
