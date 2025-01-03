use std::fmt::{self, Debug, Display, Formatter};
use std::ops::{Add, Mul, MulAssign, Neg, Range, RangeInclusive, Sub};

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
    pub const fn centipawn(cp: i16) -> Self {
        debug_assert!(CENTIPAWN_RANGE.start <= cp && cp < CENTIPAWN_RANGE.end);
        Self(cp)
    }

    /// Creates a new negative mate `Value` from a number of ply.
    #[inline]
    pub fn neg_mate_in_ply(ply: u16) -> Self {
        debug_assert!(ply <= MAX_MATE_PLY as u16);
        Self(NEG_INF.0 + ply as i16)
    }

    /// Creates a new mate `Value` from a number of ply.
    #[inline]
    pub fn mate_in_ply(ply: u16) -> Self {
        debug_assert!(ply <= MAX_MATE_PLY as u16);
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
    /// the maximum value.
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
    #[inline(never)]
    pub fn dec_mate(mut self) -> Self {
        if self.0 > NEG_INF.0 && self.0 <= NEG_INF.0 + MAX_MATE_PLY {
            self.0 -= 1;
        } else if self.0 < INF.0 && self.0 >= INF.0 - MAX_MATE_PLY {
            self.0 += 1;
        }
        self
    }

    /// Returns the inner `i16` representation of the value.
    #[inline]
    pub fn into_inner(self) -> i16 {
        self.0
    }

    /// Creates a new `Value` from the inner `i16` representation.
    ///
    /// # Safety
    /// THe value must not be `i16::MIN`.
    #[inline]
    pub unsafe fn from_inner(val: i16) -> Self {
        debug_assert_ne!(val, i16::MIN);
        Self(val)
    }

    pub fn from_i32_saturating(val: i32) -> Self {
        if val > i16::MAX as i32 {
            INF
        } else if val < i16::MIN as i32 + 1 {
            NEG_INF
        } else {
            Value(val as i16)
        }
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
        let val = self.0 as i32 * rhs as i32;
        dbg!(Self::from_i32_saturating(dbg!(val)))
    }
}

impl MulAssign<i16> for Value {
    #[inline]
    fn mul_assign(&mut self, rhs: i16) {
        *self = *self * rhs;
    }
}

impl Add for Value {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        let val = self.0 as i32 + rhs.0 as i32;
        Self::from_i32_saturating(val)
    }
}

impl Sub for Value {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        let val = self.0 as i32 - rhs.0 as i32;
        Self::from_i32_saturating(val)
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
            write!(f, "{:.2}", self.0 as f64 / 100.)
        }
    }
}
