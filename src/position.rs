use crate::types::*;
use crate::bb;
use std::fmt;
use crate::types::square_map::SquareMap;
use std::iter::FusedIterator;

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
    pub const fn get_castling_squares(col: Color) -> (Square, Square) {
        match col {
            Color::White => (Square::G1, Square::C1),
            Color::Black => (Square::G8, Square::C8),
        }
    }
    pub fn set(&mut self, fen_str: &str) -> fen_parser::Result<()> {
        *self = fen_parser::parse(fen_str)?;
        Ok(())
    }
    pub fn get_piece_bb(&self, piece: Piece) -> Bitboard {
        self.pieces.get_bb(piece)
    }
    pub fn get_occupied(&self) -> Bitboard {
        self.pieces.occupied
    }
    pub fn get_color_bb(&self, col: Color) -> Bitboard {
        if col == Color::White {
            self.pieces.white
        } else {
            self.pieces.black
        }
    }
    pub fn get_piece_at_sq(&self, sq: Square) -> Option<Piece> {
        self.pieces.array[sq]
    }
    pub fn get_to_move(&self) -> Color {
        self.to_move
    }
    pub fn get_castling_rights(&self) -> Bitboard {
        self.castling_rights.0
    }
    pub fn get_en_passant_sq(&self) -> Option<Square> {
        self.en_passant_sq
    }
    fn set_square(&mut self, piece: Piece, sq: Square) {
        self.pieces.set_square(piece, sq);
    }
    pub fn make_move(&mut self, _m: Move) {


        self.pieces.compute_bbs();
    }
    pub fn unmake_move(&mut self, _m: Move) {


        self.pieces.compute_bbs();
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.pieces.fmt(f)
    }
}

impl fmt::Debug for Position {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut p = Pieces::new();
        for col in [Color::White, Color::Black].iter().copied() {
            for (kind, board) in self.pieces.iter(col) {
                for sq in board {
                    p.array[sq] = Some(Piece(col, kind));
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
    white_boards: PieceBoards,
    black_boards: PieceBoards,
    occupied: Bitboard,
    white: Bitboard,
    black: Bitboard,
    array: SquareMap<Option<Piece>>,
}

impl Pieces {
    const fn new() -> Self {
        Pieces {
            white_boards: PieceBoards::new(),
            black_boards: PieceBoards::new(),
            occupied: Bitboard::EMPTY,
            white: Bitboard::EMPTY,
            black: Bitboard::EMPTY,
            array: SquareMap::new(None),
        }
    }
    fn get_bb(&self, piece: Piece) -> Bitboard {
        match piece.color() {
            Color::White => self.white_boards.get_board(piece.kind()),
            Color::Black => self.black_boards.get_board(piece.kind()),
        }
    }
    fn set_square(&mut self, piece: Piece, sq: Square) {
        if piece.color() == Color::White {
            self.white.set(sq);
            self.white_boards.get_mut_board(piece.kind()).set(sq);
        } else {
            self.black.set(sq);
            self.black_boards.get_mut_board(piece.kind()).set(sq);
        }
        self.array[sq] = Some(piece);
    }
    fn compute_bbs(&mut self) {
        self.occupied = self.white | self.black;
    }
    fn iter(&self, color: Color) -> BoardIter {
        if color == Color::White {
            self.white_boards.iter()
        } else {
            self.black_boards.iter()
        }
    }
}

impl fmt::Display for Pieces {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (i, row) in self.array.as_slice().chunks_exact(8).enumerate().rev() {
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

struct PieceBoards {
    pawn: Bitboard,
    knight: Bitboard,
    bishop: Bitboard,
    rook: Bitboard,
    queen: Bitboard,
    king: Bitboard,
}

impl PieceBoards {
    const fn new() -> Self {
        PieceBoards {
            pawn: Bitboard::EMPTY,
            knight: Bitboard::EMPTY,
            bishop: Bitboard::EMPTY,
            rook: Bitboard::EMPTY,
            queen: Bitboard::EMPTY,
            king: Bitboard::EMPTY,
        }
    }
    fn get_board(&self, kind: PieceType) -> Bitboard {
        match kind {
            PieceType::Pawn => self.pawn,
            PieceType::Knight => self.knight,
            PieceType::Bishop => self.bishop,
            PieceType::Rook => self.rook,
            PieceType::Queen => self.queen,
            PieceType::King => self.king,
        }
    }
    fn get_mut_board(&mut self, kind: PieceType) -> &mut Bitboard {
        match kind {
            PieceType::Pawn => &mut self.pawn,
            PieceType::Knight => &mut self.knight,
            PieceType::Bishop => &mut self.bishop,
            PieceType::Rook => &mut self.rook,
            PieceType::Queen => &mut self.queen,
            PieceType::King => &mut self.king,
        }
    }
    fn iter(&self) -> BoardIter {
        BoardIter {
            boards: self,
            next_kind: Some(PieceType::Pawn),
        }
    }
}

struct BoardIter<'a> {
    boards: &'a PieceBoards,
    next_kind: Option<PieceType>,
}

impl Iterator for BoardIter<'_> {
    type Item = (PieceType, Bitboard);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(pt) = self.next_kind {
            self.next_kind = match pt {
                PieceType::Pawn => Some(PieceType::Knight),
                PieceType::Knight => Some(PieceType::Bishop),
                PieceType::Bishop => Some(PieceType::Rook),
                PieceType::Rook => Some(PieceType::Queen),
                PieceType::Queen => Some(PieceType::King),
                PieceType::King => None,
            };
            Some((pt, self.boards.get_board(pt)))
        } else {
            None
        }
    }
}

impl FusedIterator for BoardIter<'_> { }

#[derive(Copy, Clone)]
struct CastlingRights(Bitboard);

impl CastlingRights {
    const WHITE_KS: Bitboard = bb!(Square::G1);
    const WHITE_QS: Bitboard = bb!(Square::C1);
    const BLACK_KS: Bitboard = bb!(Square::G8);
    const BLACK_QS: Bitboard = bb!(Square::C8);

    fn new(white_ks: bool, white_qs: bool, black_ks: bool, black_qs: bool) -> Self {
        let mut val = Bitboard::EMPTY;
        if white_ks { val |= Self::WHITE_KS; }
        if white_qs { val |= Self::WHITE_QS; }
        if black_ks { val |= Self::BLACK_KS; }
        if black_qs { val |= Self::BLACK_QS; }
        CastlingRights(val)
    }
    fn white_ks(self) -> bool {
        self.0 & Self::WHITE_KS != Bitboard::EMPTY
    }
    fn white_qs(self) -> bool {
        self.0 & Self::WHITE_QS != Bitboard::EMPTY
    }
    fn black_ks(self) -> bool {
        self.0 & Self::BLACK_KS != Bitboard::EMPTY
    }
    fn black_qs(self) -> bool {
        self.0 & Self::BLACK_QS != Bitboard::EMPTY
    }
}

impl fmt::Debug for CastlingRights {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}{}{}",
               if self.white_ks() { "K" } else { "" },
               if self.white_qs() { "Q" } else { "" },
               if self.black_ks() { "k" } else { "" },
               if self.black_qs() { "q" } else { "" })
    }
}