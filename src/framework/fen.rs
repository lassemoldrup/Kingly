use std::fmt::{Display, Formatter};
use std::error::Error;

pub const STARTING_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";


#[derive(Debug)]
pub struct FenParseError {
    message: &'static str,
}

impl FenParseError {
    pub(crate) fn new(message: &'static str) -> FenParseError {
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
        FenParseError::new(msg)
    }
}