use crate::position::{Position, CastlingRights};
use std::fmt::{Display, Formatter};
use std::error::Error;
use std::convert::TryFrom;
use crate::types::*;
use std::str::FromStr;

pub type Result<T> = std::result::Result<T, ParseFenError>;

#[derive(Debug)]
pub struct ParseFenError {
    message: &'static str,
}

impl ParseFenError {
    fn new(message: &'static str) -> ParseFenError {
        ParseFenError { message }
    }
}

impl Display for ParseFenError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "Invalid FEN: {}", self.message)
    }
}

impl Error for ParseFenError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

impl From<&'static str> for ParseFenError {
    fn from(msg: &'static str) -> Self {
        ParseFenError::new(msg)
    }
}

/// Parses FEN-strings, which contain 6 fields.
/// Will ignore certain flaws with the FEN-string.
pub fn parse(fen_str: &str) -> Result<Position> {
    let fields: Vec<&str> = fen_str.split_whitespace().collect();
    if fields.len() != 6 {
        return Err(ParseFenError::new("Not enough fields"));
    }
    let mut position = Position::new();

    // Field 1: piece positions
    let rows: Vec<&str> = fields[0].split('/').rev().collect();
    if rows.len() != 8 {
        return Err(ParseFenError::new("Incorrect number of rows"));
    }
    let mut sq = Square::A1 as u32;
    for (i, row) in rows.into_iter().enumerate() {
        for c in row.chars() {
            if sq / 8 != i as u32 {
                return Err(ParseFenError::new("Row formatted wrong"));
            }
            if let Some(n) = c.to_digit(10) {
                sq += n;
            } else {
                position.set_square(Piece::try_from(c)?, Square::get(sq as u8));
                sq += 1;
            }
        }
        if sq % 8 != 0 {
            return Err(ParseFenError::new("Row formatted wrong"));
        }
    };
    position.pieces.compute_bbs();

    // Field 2: who to move
    position.to_move = Color::try_from(fields[1].chars().next().unwrap())?;

    // Field 3: castling rights
    let field = fields[2];
    let mut castling = (false, false, false, false);
    if field != "-" {
        for c in field.chars() {
            match c {
                'K' => castling.0 = true,
                'Q' => castling.1 = true,
                'k' => castling.2 = true,
                'q' => castling.3 = true,
                _ => return Err(ParseFenError::new("Illegal character in castling rights")),
            }
        }
    }
    position.castling_rights = CastlingRights::new(castling.0, castling.1, castling.2, castling.3);

    // Field 4: en passant square
    let field = fields[3];
    if field != "-" {
        position.en_passant_sq = Some(Square::from_str(field)?);
    }

    // Field 5: ply (half-move) clock
    if let Ok(n) = fields[4].parse() {
        position.ply_clock = n;
    } else {
        return Err(ParseFenError::new("Ply clock must be a number"))
    }

    // Field 6: full-move number
    if let Ok(n) = fields[5].parse() {
        position.fullmove_number = n;
    } else {
        return Err(ParseFenError::new("Move number must be a number"))
    }

    Ok(position)
}