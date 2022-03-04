use std::fmt::{Display, self};
use std::ops::{Neg, RangeInclusive};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Value(i16);

impl Value {
    pub fn from_cp(cp: i16) -> Self {
        Self(cp)
    }

    pub fn from_neg_inf(moves: u16) -> Self {
        Self(i16::MIN + 1 + moves as i16)
    }

    pub fn from_inf(moves: u16) -> Self {
        Self(i16::MAX - moves as i16)
    }
}

impl Neg for Value {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self(-self.0)
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