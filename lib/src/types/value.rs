use std::fmt::{self, Debug, Display, Formatter};
use std::ops::{Add, Mul, Neg, RangeInclusive, Sub};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Value(i16);

impl Value {
    const NEG_INF: RangeInclusive<i16> = (i16::MIN + 1)..=(i16::MIN + 1 + 100);
    const INF: RangeInclusive<i16> = (i16::MAX - 100)..=i16::MAX;

    pub const fn centi_pawn(cp: i16) -> Self {
        Self(cp)
    }

    pub const fn mate_in_ply_neg(moves: u16) -> Self {
        Self(i16::MIN + 1 + moves as i16)
    }

    pub const fn mate_in_ply(moves: u16) -> Self {
        Self(i16::MAX - moves as i16)
    }

    pub fn inc_mate(mut self) -> Self {
        if self.0 <= i16::MIN + 1 + 100 {
            self.0 += 1;
        } else if self.0 >= i16::MAX - 100 {
            self.0 -= 1;
        }
        self
    }

    pub fn dec_mate(mut self) -> Self {
        if self.0 <= i16::MIN + 1 + 100 && self.0 != i16::MIN + 1 {
            self.0 -= 1;
        } else if self.0 >= i16::MAX - 100 && self.0 != i16::MAX {
            self.0 += 1;
        }
        self
    }

    pub fn into_inner(self) -> i16 {
        self.0
    }
}

impl Neg for Value {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}

impl Add for Value {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0.saturating_add(rhs.0).max(i16::MIN + 1))
    }
}

impl Sub for Value {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0.saturating_sub(rhs.0).max(i16::MIN + 1))
    }
}

impl From<i16> for Value {
    fn from(value: i16) -> Self {
        Self(value)
    }
}

impl Mul<i16> for Value {
    type Output = Self;

    fn mul(self, rhs: i16) -> Self::Output {
        Self(self.0.saturating_mul(rhs))
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self.0 {
            v if Self::NEG_INF.contains(&v) => write!(f, "mate -{}", (v - i16::MIN) / 2),
            v if Self::INF.contains(&v) => write!(f, "mate {}", (i16::MAX - v + 1) / 2),
            v => write!(f, "cp {}", v),
        }
    }
}

impl Debug for Value {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self.0 {
            v if Self::NEG_INF.contains(&v) => write!(f, "-m{} (ply)", v - i16::MIN - 1),
            v if Self::INF.contains(&v) => write!(f, "m{} (ply)", i16::MAX - v),
            v => write!(f, "{}.{:02}", v / 100, (v % 100).abs()),
        }
    }
}
