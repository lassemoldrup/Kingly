use std::fmt::{self, Debug, Display, Formatter};
use std::mem;
use std::str::FromStr;

use super::{ParseSquareError, PieceFromCharError, PieceKind, Square};

/// Used to create a [`Move`].
///
/// # Examples
/// ```
/// use kingly_lib::mv;
/// use kingly_lib::types::{Move, Square, PieceKind};
///
/// assert_eq!(mv!(E2 -> E4), Move::new_regular(Square::E2, Square::E4, false));
/// assert_eq!(mv!(E4 x D5), Move::new_regular(Square::E4, Square::D5, true));
/// assert_eq!(mv!(O-O w), Move::new_castling(Square::E1, Square::G1));
/// assert_eq!(mv!(O-O-O b), Move::new_castling(Square::E8, Square::C8));
/// assert_eq!(mv!(E7 -> E8 q), Move::new_promotion(Square::E7, Square::E8, PieceKind::Queen, false));
/// assert_eq!(mv!(B2 x A1 n), Move::new_promotion(Square::B2, Square::A1, PieceKind::Knight, true));
/// assert_eq!(mv!(E5 ep D6), Move::new_en_passant(Square::E5, Square::D6));
/// assert_eq!(mv!(), Move::NULL);
/// ```
#[macro_export]
macro_rules! mv {
    ( $from:tt -> $to:tt ) => {{
        #[allow(unused_imports)]
        use $crate::types::Square::*;
        $crate::types::Move::new_regular($from, $to, false)
    }};
    ( $from:tt x $to:tt ) => {{
        #[allow(unused_imports)]
        use $crate::types::Square::*;
        $crate::types::Move::new_regular($from, $to, true)
    }};
    ( O-O w ) => {{
        use $crate::types::Square::*;
        $crate::types::Move::new_castling(E1, G1)
    }};
    ( O-O-O w ) => {{
        use $crate::types::Square::*;
        $crate::types::Move::new_castling(E1, C1)
    }};
    ( O-O b ) => {{
        use $crate::types::Square::*;
        $crate::types::Move::new_castling(E8, G8)
    }};
    ( O-O-O b ) => {{
        use $crate::types::Square::*;
        $crate::types::Move::new_castling(E8, C8)
    }};
    ( $from:tt ep $to:tt ) => {{
        #[allow(unused_imports)]
        use $crate::types::Square::*;
        $crate::types::Move::new_en_passant($from, $to)
    }};
    ( $from:tt -> $to:tt $kind:tt ) => {{
        #[allow(unused_imports)]
        use $crate::types::Square::*;
        $crate::types::Move::new_promotion($from, $to, mv!(@kind $kind), false)
    }};
    ( $from:tt x $to:tt $kind:tt ) => {{
        #[allow(unused_imports)]
        use $crate::types::Square::*;
        $crate::types::Move::new_promotion($from, $to, mv!(@kind $kind), true)
    }};
    () => { $crate::types::Move::NULL };
    (@kind n) => {
        $crate::types::PieceKind::Knight
    };
    (@kind b) => {
        $crate::types::PieceKind::Bishop
    };
    (@kind r) => {
        $crate::types::PieceKind::Rook
    };
    (@kind q) => {
        $crate::types::PieceKind::Queen
    };
    (@kind $kind:expr) => {
        $kind
    };
}

/// A chess move which includes information on squares, capture, move kind, and
/// promotion.
// Bit layout: ki|p|c|....to|..from
// 0-5: from sq
// 6-11: to sq
// 12: capture
// 13: promotion
// 14-15: kind (0: regular, 1: castling, 2: promotion (unused) 3: en passant) OR
// 14-15: promotion (0: knight, 1: bishop, 2: Rook, 3: Queen)
// The all-0s move is not a valid move, as from sq and to sq cannot be the same, so it is
// used to represent a null move.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Move(u16);

impl Move {
    /// The null move.
    pub const NULL: Self = Self(0);

    /// Creates a new regular move.
    #[inline]
    pub const fn new_regular(from: Square, to: Square, capture: bool) -> Self {
        let mut encoding = 0;

        encoding |= from as u16;
        encoding |= (to as u16) << 6;
        encoding |= (capture as u16) << 12;

        Self(encoding)
    }

    /// Creates a new castling move.
    #[inline]
    pub const fn new_castling(from: Square, to: Square) -> Self {
        let mut encoding = 1 << 14;

        encoding |= from as u16;
        encoding |= (to as u16) << 6;

        Self(encoding)
    }

    /// Creates a new promotion move.
    #[inline]
    pub const fn new_promotion(from: Square, to: Square, kind: PieceKind, capture: bool) -> Self {
        let mut encoding = 1 << 13;

        encoding |= from as u16;
        encoding |= (to as u16) << 6;
        encoding |= (capture as u16) << 12;
        encoding |= (kind as u16) << 14;

        Self(encoding)
    }

    /// Creates a new en passant move.
    #[inline]
    pub const fn new_en_passant(from: Square, to: Square) -> Self {
        let mut encoding = (1 << 12) | (3 << 14);

        encoding |= from as u16;
        encoding |= (to as u16) << 6;

        Self(encoding)
    }

    /// Returns the from square of the move.
    #[inline]
    pub const fn from(self) -> Square {
        // Safety: since the value transmuted is 6 bits, it's in [0; 63]
        unsafe { Square::from_unchecked((self.0 & 0b111111) as u8) }
    }

    /// Returns the to square of the move.
    #[inline]
    pub const fn to(self) -> Square {
        // Safety: since the value transmuted is 6 bits, it's in [0; 63]
        unsafe { Square::from_unchecked(((self.0 >> 6) & 0b111111) as u8) }
    }

    /// Returns whether the move is a capture.
    #[inline]
    pub const fn capture(self) -> bool {
        self.0 & (1 << 12) != 0
    }

    /// Returns the [`MoveKind`] of the move.
    #[inline]
    pub const fn kind(self) -> MoveKind {
        // Kind could either be a move kind or a promotion kind
        let kind = ((self.0 >> 14) & 0b11) as u16;
        if self.0 & (1 << 13) != 0 {
            // Safety: since kind is 2 bits, it's safe to transmute to a PieceKind
            MoveKind::Promotion(unsafe { mem::transmute(kind as u8) })
        } else {
            // Safety: MoveKind is repr(C, u8), which means it is represented as a
            // repr(C) struct where the first field is a u8 tag, which is then
            // placed at the first 8 bits of the representation of the struct.
            // This means that it is safe to transmute kind to a MoveKind.
            unsafe { mem::transmute(kind) }
        }
    }

    /// Returns the promotion kind of the move, if it is a promotion.
    #[inline]
    pub const fn promotion(self) -> Option<PieceKind> {
        if self.0 & (1 << 13) != 0 {
            let kind = ((self.0 >> 14) & 0b11) as u8;
            Some(unsafe { mem::transmute(kind) })
        } else {
            None
        }
    }

    /// Parses a move from a string in the UCI format, e.g. `e2e4` and `g2g1q`,
    /// given a list of legal moves.
    pub fn parse_from_legal(value: &str, legal_moves: &[Move]) -> Result<Self, TryFromLegalError> {
        PseudoMove::from_str(value)?
            .into_move(legal_moves)
            .map_err(TryFromLegalError::from)
    }

    /// Returns whether the move is the null move.
    #[inline]
    pub fn is_null(self) -> bool {
        self == Self::NULL
    }

    /// Returns the inner `u16` representation of the move.
    #[inline]
    pub const fn into_inner(self) -> u16 {
        self.0
    }
}

impl From<Move> for u16 {
    #[inline]
    fn from(mv: Move) -> Self {
        mv.0
    }
}

impl Display for Move {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if self.is_null() {
            return write!(f, "NULL");
        }
        match self.kind() {
            MoveKind::Regular | MoveKind::Castling | MoveKind::EnPassant => {
                write!(f, "{}{}", self.from(), self.to())
            }
            MoveKind::Promotion(kind) => write!(f, "{}{}{}", self.from(), self.to(), kind),
        }
    }
}

impl Debug for Move {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if self.is_null() {
            return write!(f, "NULL");
        }
        match self.kind() {
            MoveKind::Castling => write!(f, "c{}", self),
            MoveKind::EnPassant => write!(f, "ep{}", self),
            _ => write!(f, "{}", self),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum TryFromLegalError {
    #[error("{0}")]
    Parse(#[from] ParseMoveError),
    #[error("{0}")]
    Illegal(#[from] IllegalMoveError),
}

/// Represents the kind of a move, i.e. regular, castling, promotion, or en
/// passant.
#[repr(C, u8)]
pub enum MoveKind {
    /// A regular move.
    Regular,
    /// A castling move.
    Castling,
    /// A promotion move, including the type of promotion.
    Promotion(PieceKind),
    /// An en passant move.
    EnPassant,
}

/// Represents a move (from square, to square, optional promotion) without
/// extra information such as whether it's a castling move or a capture.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct PseudoMove {
    pub from: Square,
    pub to: Square,
    pub promotion: Option<PieceKind>,
}

impl PseudoMove {
    #[inline]
    pub const fn new(from: Square, to: Square, promotion: Option<PieceKind>) -> Self {
        Self {
            from,
            to,
            promotion,
        }
    }

    pub fn into_move(self, legal_moves: &[Move]) -> Result<Move, IllegalMoveError> {
        legal_moves
            .iter()
            .copied()
            .find(|&mv| {
                mv.from() == self.from && mv.to() == self.to && mv.promotion() == self.promotion
            })
            .ok_or_else(|| IllegalMoveError(self))
    }
}

impl From<Move> for PseudoMove {
    fn from(mv: Move) -> Self {
        Self {
            from: mv.from(),
            to: mv.to(),
            promotion: mv.promotion(),
        }
    }
}

impl PartialEq<Move> for PseudoMove {
    fn eq(&self, other: &Move) -> bool {
        self.from == other.from() && self.to == other.to() && self.promotion == other.promotion()
    }
}

impl FromStr for PseudoMove {
    type Err = ParseMoveError;

    /// Parses a move from a string in the UCI format, e.g. `e2e4` and `g2g1q`.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() < 4 || s.len() > 5 {
            return Err(ParseMoveError::InvalidLength);
        }

        let from = s[0..2].parse()?;
        let to = s[2..4].parse()?;
        let promotion = s.chars().nth(4).map(PieceKind::try_from).transpose()?;

        Ok(PseudoMove {
            from,
            to,
            promotion,
        })
    }
}

impl Display for PseudoMove {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}{}", self.from, self.to)
    }
}

#[derive(thiserror::Error, Debug)]
#[error("Illegal move '{0}'")]
pub struct IllegalMoveError(pub PseudoMove);

#[derive(thiserror::Error, Debug)]
pub enum ParseMoveError {
    #[error("length of move is not valid")]
    InvalidLength,
    #[error("invalid square in move: {0}")]
    InvalidSquare(#[from] ParseSquareError),
    #[error("promotion is not valid: {0}")]
    InvalidPromotion(#[from] PieceFromCharError),
}
