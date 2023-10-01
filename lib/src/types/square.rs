use std::str::FromStr;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Square;

#[derive(thiserror::Error, Debug)]
#[error("Invalid square")]
pub struct ParseSquareError;

// TDOO: Implement FromStr for Square
impl FromStr for Square {
    type Err = ParseSquareError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() == 2 {
            let file = s.chars().nth(0).unwrap();
            let rank = s.chars().nth(1).unwrap();
            if file.is_ascii_alphabetic() && rank.is_ascii_digit() {
                Ok(Square)
            } else {
                Err(ParseSquareError)
            }
        } else {
            Err(ParseSquareError)
        }
    }
}
