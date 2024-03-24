use std::num::ParseIntError;
use std::str::FromStr;

use intmap::IntMap;
use strum::IntoEnumIterator;

use crate::tables::Tables;
use crate::types::{
    CastlingRights, Color, File, ParseSquareError, Piece, PieceFromCharError, PieceKind, Rank,
    Square,
};
use crate::zobrist::ZobristKey;

use super::pieces::Pieces;
use super::Position;

pub const STARTING_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

#[derive(thiserror::Error, Debug)]
pub enum ParseFenError {
    #[error("incorrect number of FEN fields: expected 6, got {0}")]
    IncorrectFieldCount(usize),
    #[error("incorrect number of ranks in FEN string: expected 8, got {0}")]
    IncorrectRankCount(usize),
    #[error("too many files in rank '{0}'")]
    TooManyFiles(String),
    #[error("too few files in rank '{0}'")]
    TooFewFiles(String),
    #[error("{0}")]
    InvalidPiece(#[from] PieceFromCharError),
    #[error("each player must have exactly one king")]
    IncorrectKingCount,
    #[error("invalid player color '{0}'")]
    InvalidColor(String),
    #[error("invalid castling rights '{0}'")]
    InvalidCastlingRights(String),
    #[error("invalid en passant square: {0}")]
    InvalidEnPassantSquare(#[from] ParseSquareError),
    #[error("invalid ply clock: {0}")]
    InvalidPlyClock(ParseIntError),
    #[error("invalid move number: {0}")]
    InvalidMoveNumber(ParseIntError),
}

impl FromStr for Position {
    type Err = ParseFenError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Position::from_fen(s)
    }
}

impl Position {
    /// Creates a position from a FEN string
    pub fn from_fen(fen: &str) -> Result<Self, ParseFenError> {
        let fields: Vec<&str> = fen.split(' ').collect();
        if fields.len() != 6 {
            return Err(ParseFenError::IncorrectFieldCount(fields.len()));
        }

        // Piece placement
        let mut pieces = Pieces::new();
        let ranks: Vec<&str> = fields[0].split('/').rev().collect();
        if ranks.len() != 8 {
            return Err(ParseFenError::IncorrectRankCount(ranks.len()));
        }
        for (rank, rank_str) in Rank::iter().zip(ranks) {
            let mut f = 0;
            for ch in rank_str.chars() {
                let file = File::from_repr(f)
                    .ok_or_else(|| ParseFenError::TooManyFiles(rank_str.to_string()))?;
                if let Some(n) = ch.to_digit(10) {
                    f += n as u8;
                } else {
                    let sq = Square::from_rank_file(rank, file);
                    pieces.set_sq(sq, Piece::try_from(ch)?);
                    f += 1;
                }
            }
            if f > 8 {
                return Err(ParseFenError::TooManyFiles(rank_str.to_string()));
            } else if f < 8 {
                return Err(ParseFenError::TooFewFiles(rank_str.to_string()));
            }
        }
        if pieces.get_bb(Piece(PieceKind::King, Color::White)).len() != 1
            || pieces.get_bb(Piece(PieceKind::King, Color::Black)).len() != 1
        {
            return Err(ParseFenError::IncorrectKingCount);
        }

        // Player to move
        let to_move_str = fields[1];
        let to_move = match to_move_str {
            "w" => Color::White,
            "b" => Color::Black,
            _ => return Err(ParseFenError::InvalidColor(to_move_str.to_string())),
        };

        // Castling rights
        let castling_str = fields[2];
        let mut castling = CastlingRights::new(false, false, false, false);
        if castling_str != "-" {
            for right in castling_str.chars() {
                match right {
                    'K' => castling.set(Color::White, 0b01),
                    'Q' => castling.set(Color::White, 0b10),
                    'k' => castling.set(Color::Black, 0b01),
                    'q' => castling.set(Color::Black, 0b10),
                    _ => {
                        return Err(ParseFenError::InvalidCastlingRights(
                            castling_str.to_string(),
                        ))
                    }
                }
            }
            if u8::from(castling).count_ones() != castling_str.len() as u32 {
                return Err(ParseFenError::InvalidCastlingRights(
                    castling_str.to_string(),
                ));
            }
        }

        // En passant square
        let en_passant_sq_str = fields[3];
        let en_passant_sq = match en_passant_sq_str {
            "-" => None,
            _ => Some(Square::from_str(en_passant_sq_str)?),
        };

        // Ply clock
        let ply_clock = fields[4].parse().map_err(ParseFenError::InvalidPlyClock)?;

        // Move number
        let move_number = fields[5]
            .parse()
            .map_err(ParseFenError::InvalidMoveNumber)?;

        // Zobrist hash
        let tables = Tables::get();
        let mut zobrist = 0;

        for pce in Piece::iter() {
            zobrist ^= (pce, pieces.get_bb(pce)).key(tables);
        }
        zobrist ^= to_move.key(tables);
        zobrist ^= castling.key(tables);
        zobrist ^= en_passant_sq.key(tables);

        // Repetition table
        let mut repetitions = IntMap::new();
        repetitions.insert(zobrist, 1);

        Ok(Position {
            pieces,
            to_move,
            castling,
            en_passant_sq,
            ply_clock,
            move_number,
            repetitions,
            history: Vec::new(),
            zobrist,
            tables,
        })
    }
}
