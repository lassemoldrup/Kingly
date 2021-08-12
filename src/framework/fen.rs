use std::error::Error;
use std::fmt::{Display, Formatter};

pub const STARTING_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";


#[derive(Debug)]
pub struct FenParseError {
    message: String,
}

impl FenParseError {
    pub(crate) fn new(message: String) -> FenParseError {
        FenParseError { message }
    }
}

impl Display for FenParseError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "Invalid FEN: {}", self.message)
    }
}

impl Error for FenParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

impl From<&'static str> for FenParseError {
    fn from(msg: &'static str) -> Self {
        FenParseError::new(String::from(msg))
    }
}

impl From<String> for FenParseError {
    fn from(msg: String) -> Self {
        FenParseError::new(msg)
    }
}