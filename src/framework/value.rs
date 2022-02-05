use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use std::ops::Neg;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Value {
    NegInf(u32),
    CentiPawn(i32),
    Inf(u32),
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Value {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::CentiPawn(v1), Self::CentiPawn(v2)) => v1.cmp(v2),
            (Self::NegInf(mvs1), Self::NegInf(mvs2))
            | (Self::Inf(mvs2), Self::Inf(mvs1)) => mvs1.cmp(mvs2),
            (Self::CentiPawn(_), Self::Inf(_))
            | (Self::NegInf(_), _) => Ordering::Less,
            (Self::CentiPawn(_), Self::NegInf(_))
            | (Self::Inf(_), _) => Ordering::Greater,
        }
    }
}

impl Neg for Value {
    type Output = Value;

    fn neg(self) -> Self::Output {
        match self {
            Self::NegInf(moves) => Self::Inf(moves),
            Self::CentiPawn(val) => Self::CentiPawn(-val),
            Self::Inf(moves) => Self::NegInf(moves),
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            // TODO
            Self::NegInf(moves) => write!(f, "mate -{}", moves),
            Self::CentiPawn(val) => write!(f, "cp {}", val),
            Self::Inf(moves) => write!(f, "mate {}", moves),
        }
    }
}