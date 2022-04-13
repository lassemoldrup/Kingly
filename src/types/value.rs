use std::fmt::{self, Display};
use std::ops::{Add, Mul, Neg, RangeInclusive, Sub};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Value(i16);

impl Value {
    pub const fn centi_pawn(cp: i16) -> Self {
        Self(cp)
    }

    pub const fn mate_in_neg(moves: u16) -> Self {
        Self(i16::MIN + 1 + moves as i16)
    }

    pub const fn mate_in(moves: u16) -> Self {
        Self(i16::MAX - moves as i16)
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

impl Mul<i16> for Value {
    type Output = Self;

    fn mul(self, rhs: i16) -> Self::Output {
        Self(self.0.saturating_mul(rhs))
    }
}

const NEG_INF: RangeInclusive<i16> = (i16::MIN + 1)..=(i16::MIN + 1 + 100);
const INF: RangeInclusive<i16> = (i16::MAX - 100)..=i16::MAX;

impl Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            v if NEG_INF.contains(&v) => write!(f, "mate -{}", v - i16::MIN - 1),
            v if INF.contains(&v) => write!(f, "mate {}", i16::MAX - v),
            v => write!(f, "cp {}", v),
        }
    }
}
