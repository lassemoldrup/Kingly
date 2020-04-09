use crate::types::*;
use std::fmt;
use std::fmt::Display;
use enum_map::{EnumMap, Enum};

mod fen_parser;

const DEFAULT_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

pub struct Position {
    pieces: Pieces,
    to_move: Color,
    castling_rights: CastlingRights,
    en_passant_sq: Option<Square>,
    ply_clock: u32,
    fullmove_number: u32,
}

impl Position {
    fn new() -> Self {
        Position {
            pieces: Pieces::new(),
            to_move: Color::White,
            castling_rights: CastlingRights::new(false, false, false, false),
            en_passant_sq: None,
            ply_clock: 0,
            fullmove_number: 1,
        }
    }
    pub fn new_default() -> Self {
        fen_parser::parse(DEFAULT_FEN).unwrap()
    }
    pub fn new_from(fen_str: &str) -> fen_parser::Result<Self> {
        fen_parser::parse(fen_str)
    }
    pub fn get_piece_bitboard(&self, piece: Piece) -> Bitboard {
        self.pieces.get_bitboard(piece)
    }
    pub fn get_castling_rights(&self, col: Color, side: Side) -> bool {
        match (col, side) {
            (Color::White, Side::KingSide) => self.castling_rights.white_ks,
            (Color::White, Side::QueenSide) => self.castling_rights.white_qs,
            (Color::Black, Side::KingSide) => self.castling_rights.black_ks,
            (Color::Black, Side::QueenSide) => self.castling_rights.black_qs,
        }
    }
    fn set_square(&mut self, piece: Piece, sq: Square) {
        self.pieces.set_square(piece, sq);
    }
    pub fn make_move(&mut self, m: Move) {

    }
    pub fn unmake_move(&mut self, m: Move) {

    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.pieces.fmt(f)
    }
}

impl fmt::Debug for Position {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut p = Pieces {
            boards: EnumMap::new(),
            array: EnumMap::new(),
        };
        for (col, pce_boards) in self.pieces.boards {
            for (kind, board) in pce_boards {
                for sq in board {
                    p.array[sq] = Some(Piece::new(kind, col));
                }
            }
        }
        writeln!(f, "Bitboards:{}", p)?;
        writeln!(f, "Piece array:{}", self.pieces)?;
        writeln!(f, "To move:\t\t\t{:?}", self.to_move)?;
        writeln!(f, "Castling rights:\t{:?}", self.castling_rights)?;
        writeln!(f, "En passant square:\t{:?}", self.en_passant_sq)?;
        writeln!(f, "Ply clock:\t\t\t{}", self.ply_clock)?;
        writeln!(f, "Full-move number:\t{}", self.fullmove_number)
    }
}

struct Pieces {
    boards: EnumMap<Color, EnumMap<PieceType, Bitboard>>,
    array: EnumMap<Square, Option<Piece>>,
}

impl Pieces {
    fn new() -> Self {
        Pieces {
            boards: EnumMap::new(),
            array: EnumMap::new(),
        }
    }
    fn get_bitboard(&self, piece: Piece) -> Bitboard {
        self.boards[piece.color][piece.kind]
    }
    fn set_square(&mut self, piece: Piece, sq: Square) {
        self.boards[piece.color][piece.kind].set(sq);
        self.array[sq] = Some(piece);
    }
}

impl fmt::Display for Pieces {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (i, row) in self.array.as_slice().chunks(8).enumerate().rev() {
            writeln!(f, "\n\t  +---+---+---+---+---+---+---+---+")?;
            write!(f, "\t{} | ", i + 1)?;
            for s in row {
                match s {
                    Some(p) => write!(f, "{} | ", p)?,
                    None => write!(f, "  | ")?,
                }
            }
        }
        writeln!(f, "\n\t  +---+---+---+---+---+---+---+---+")?;
        write!(f, "\t    A   B   C   D   E   F   G   H")
    }
}

struct CastlingRights {
    white_ks: bool,
    white_qs: bool,
    black_ks: bool,
    black_qs: bool,
}

impl CastlingRights {
    fn new(white_ks: bool, white_qs: bool, black_ks: bool, black_qs: bool) -> CastlingRights {
        CastlingRights { white_ks, white_qs, black_ks, black_qs }
    }
}

impl fmt::Debug for CastlingRights {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}{}{}",
               if self.white_ks { "K" } else { "" },
               if self.white_qs { "Q" } else { "" },
               if self.black_ks { "k" } else { "" },
               if self.black_qs { "q" } else { "" })
    }
}