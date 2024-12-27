use std::fmt::{Debug, Display, Formatter};
use std::ops::Index;

use strum::IntoEnumIterator;

use crate::bb;
use crate::collections::SquareMap;
use crate::types::{Bitboard, Color, File, Piece, PieceKind, Rank, Square};

/// Holds the placement of pieces on the board.
/// Allows for quickly querying [`Bitboard`]s of pieces and occupied squares as
/// well as getting and setting pieces at specific squares on the board.
#[derive(PartialEq, Debug, Clone, Default)]
pub struct Pieces {
    white_pieces: PieceBoards,
    black_pieces: PieceBoards,
    map: SquareMap<Option<Piece>>,
}

impl Pieces {
    /// Creates a new `Pieces` instance with no pieces on the board. Note: this
    /// is not a legal chess position, since there are no kings.
    #[inline]
    pub const fn new() -> Self {
        Self {
            white_pieces: PieceBoards::new(),
            black_pieces: PieceBoards::new(),
            map: SquareMap::new([None; 64]),
        }
    }

    /// Gets the [`Piece`] on the given [`Square`].
    #[inline]
    pub fn get(&self, sq: Square) -> Option<Piece> {
        self.map[sq]
    }

    /// Gets the [`Bitboard`] of the given [`Piece`].
    #[inline]
    pub const fn get_bb(&self, pce: Piece) -> Bitboard {
        match pce.color() {
            Color::White => self.white_pieces.get(pce.kind()),
            Color::Black => self.black_pieces.get(pce.kind()),
        }
    }

    /// Gets the [`Bitboard`] of all occupied squares for the given [`Color`].
    #[inline]
    pub const fn occupied_for(&self, color: Color) -> Bitboard {
        match color {
            Color::White => self.white_pieces.occupied,
            Color::Black => self.black_pieces.occupied,
        }
    }

    /// Gets the [`Square`] of the king for the given [`Color`].
    ///
    /// # Panics
    /// Panics if there is no king of the given color.
    #[inline]
    pub fn king_sq_for(&self, color: Color) -> Square {
        let king = self.get_bb(Piece(PieceKind::King, color));
        king.into_iter().next().expect("there should be a king")
    }

    /// Gets a [`Bitboard`] of all occupied squares.
    #[inline]
    pub fn occupied(&self) -> Bitboard {
        self.white_pieces.occupied | self.black_pieces.occupied
    }

    /// Sets the given [`Piece`] on the given [`Square`].
    #[inline]
    pub fn set_sq(&mut self, sq: Square, pce: Piece) {
        match pce.color() {
            Color::White => self.white_pieces.set_sq(pce.kind(), sq),
            Color::Black => self.black_pieces.set_sq(pce.kind(), sq),
        }

        self.map[sq] = Some(pce);
    }

    /// Unsets the given [`Square`].
    #[inline]
    pub fn unset_sq(&mut self, sq: Square) {
        let bb = bb!(sq);
        self.white_pieces.unset_sqs(bb);
        self.black_pieces.unset_sqs(bb);
        self.map[sq] = None;
    }

    /// Returns the number of pieces on the board.
    #[inline]
    pub fn count(&self) -> usize {
        self.occupied().len()
    }
}

impl Index<Square> for Pieces {
    type Output = Option<Piece>;

    #[inline]
    fn index(&self, index: Square) -> &Self::Output {
        &self.map[index]
    }
}

impl Display for Pieces {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f)?;
        for rank in Rank::iter().rev() {
            for file in File::iter() {
                let sq = Square::from_rank_file(rank, file);
                match self[sq] {
                    None => write!(f, ". ")?,
                    Some(pce) => write!(f, "{} ", pce)?,
                }
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

#[derive(PartialEq, Debug, Copy, Clone, Default)]
struct PieceBoards {
    boards: [Bitboard; 6],
    occupied: Bitboard,
}

impl PieceBoards {
    const fn new() -> Self {
        Self {
            boards: [bb!(); 6],
            occupied: bb!(),
        }
    }

    const fn get(&self, kind: PieceKind) -> Bitboard {
        self.boards[kind as usize]
    }

    fn set_sq(&mut self, kind: PieceKind, sq: Square) {
        self.boards[kind as usize].add_sq(sq);
        self.occupied.add_sq(sq);
    }

    fn unset_sqs(&mut self, bb: Bitboard) {
        for board in &mut self.boards {
            *board -= bb;
        }
        self.occupied -= bb;
    }
}
