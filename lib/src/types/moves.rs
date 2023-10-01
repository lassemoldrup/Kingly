use std::str::FromStr;

use super::{ParseSquareError, Square};

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct PseudoMove {
    from: Square,
    to: Square,
    // TODO
    promotion: Option<()>,
}

#[derive(thiserror::Error, Debug)]
pub enum ParsePseudoMoveError {
    #[error("Invalid move: Length is not valid")]
    InvalidLength,
    #[error("Invalid move: {0}")]
    InvalidSquare(#[from] ParseSquareError),
    #[error("Invalid move: Promotion is not valid")]
    InvalidPromotion,
}

impl FromStr for PseudoMove {
    type Err = ParsePseudoMoveError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !(s.len() == 4 || s.len() == 5) {
            return Err(ParsePseudoMoveError::InvalidLength);
        }

        let from = s[0..2].parse()?;
        let to = s[2..4].parse()?;
        let promotion = if s.len() == 5 { Some(()) } else { None };

        Ok(PseudoMove {
            from,
            to,
            promotion,
        })
    }
}
