use std::fmt::{self, Debug, Display, Formatter};
use std::ops::{Mul, Neg, Range, RangeInclusive};

/// A value of positive infinity.
pub const INF: Value = Value(i16::MAX);
/// A value of negative infinity.
pub const NEG_INF: Value = Value(i16::MIN + 1);
const MAX_MATE_PLY: i16 = 100;
const CENTIPAWN_RANGE: Range<i16> = (NEG_INF.0 + MAX_MATE_PLY + 1)..(INF.0 - MAX_MATE_PLY);
const NEG_MATE_RANGE: RangeInclusive<i16> = NEG_INF.0..=(NEG_INF.0 + MAX_MATE_PLY);
const MATE_RANGE: RangeInclusive<i16> = (INF.0 - MAX_MATE_PLY)..=INF.0;

/// A value representing a score or a number of ply until mate.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Value(i16);

impl Value {
    /// Creates a new `Value` from a centipawn value.
    #[inline]
    pub fn centipawn(cp: i16) -> Self {
        debug_assert!(CENTIPAWN_RANGE.contains(&cp));
        Self(cp)
    }

    /// Creates a new negative mate `Value` from a number of ply.
    #[inline]
    pub fn neg_mate_in_ply(ply: u16) -> Self {
        Self(NEG_INF.0 + ply as i16)
    }

    /// Creates a new mate `Value` from a number of ply.
    #[inline]
    pub fn mate_in_ply(ply: u16) -> Self {
        Self(INF.0 - ply as i16)
    }

    /// Returns whether the value is a centipawn value.
    #[inline]
    pub fn is_centipawn(self) -> bool {
        CENTIPAWN_RANGE.contains(&self.0)
    }

    /// Returns whether the value is a (positive) mate value.
    #[inline]
    pub fn is_mate(self) -> bool {
        MATE_RANGE.contains(&self.0)
    }

    /// Returns whether the value is a negative mate value.
    #[inline]
    pub fn is_neg_mate(self) -> bool {
        NEG_MATE_RANGE.contains(&self.0)
    }

    /// Returns a value with the mate-in-ply incremented by one.
    /// If the resulting value would overflow the mate range, it is clamped to
    /// the maximum value..
    #[inline]
    pub fn inc_mate(mut self) -> Self {
        if self.0 < NEG_INF.0 + MAX_MATE_PLY {
            self.0 += 1;
        } else if self.0 > INF.0 - MAX_MATE_PLY {
            self.0 -= 1;
        }
        self
    }

    /// Returns a value with the mate-in-ply decremented by one.
    /// If the resulting value would overflow the mate range, it is clamped to
    /// the minimum value.
    #[inline]
    pub fn dec_mate(mut self) -> Self {
        if self.is_neg_mate() && self != NEG_INF {
            self.0 -= 1;
        } else if self.is_mate() && self != INF {
            self.0 += 1;
        }
        self
    }
}

impl Neg for Value {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}

impl Mul<i16> for Value {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: i16) -> Self::Output {
        Self(self.0.saturating_mul(rhs))
    }
}

impl From<i16> for Value {
    #[inline]
    fn from(val: i16) -> Self {
        Self(val)
    }
}

impl From<Value> for i16 {
    #[inline]
    fn from(val: Value) -> i16 {
        val.0
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if self.is_neg_mate() {
            write!(f, "mate -{}", (self.0 - i16::MIN) / 2)
        } else if self.is_mate() {
            write!(f, "mate {}", (i16::MAX - self.0 + 1) / 2)
        } else {
            write!(f, "cp {}", self.0)
        }
    }
}

impl Debug for Value {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if self.is_neg_mate() {
            write!(f, "-m{} (ply)", self.0 - i16::MIN - 1)
        } else if self.is_mate() {
            write!(f, "m{} (ply)", i16::MAX - self.0)
        } else {
            write!(f, "{}.{:02}", self.0 / 100, (self.0 % 100).abs())
        }
    }
}
