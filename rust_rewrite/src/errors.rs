use std::{io, num::{ParseFloatError, ParseIntError, TryFromIntError}, str::Utf8Error};
use quick_xml::events::attributes::AttrError;
use png::DecodingError;

#[derive(Debug)] 
pub struct Error {
    pub message: String,
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

impl From<ParseFloatError> for Error {
    fn from(value: ParseFloatError) -> Self {
        Error {
            message: value.to_string()
        }
    }
}

impl From<ParseIntError> for Error {
    fn from(value: ParseIntError) -> Self {
        Error {
            message: value.to_string()
        }
    }
}

impl From<AttrError> for Error {
    fn from(value: AttrError) -> Self {
        Error {
            message: value.to_string()
        }
    }
}

impl From<&AttrError> for Error {
    fn from(value: &AttrError) -> Self {
        Error {
            message: value.to_string()
        }
    }
}

impl From<Utf8Error> for Error {
    fn from(value: Utf8Error) -> Self {
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

impl From<String> for Error {
    fn from(value: String) -> Self {
        Error {
            message: value
        }
    }
}

impl From<TryFromIntError> for Error {
    fn from(value: TryFromIntError) -> Self {
        Error {
            message: value.to_string()
        }
    }
}

impl From<DecodingError> for Error {
    fn from(value: DecodingError) -> Self {
        Error {
            message: value.to_string()
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
