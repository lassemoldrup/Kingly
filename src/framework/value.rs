use std::fmt::{Display, Formatter};
use std::ops::Neg;

#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
pub enum Value {
    NegInf,
    CentiPawn(i32),
    Inf,
}

impl Neg for Value {
    type Output = Value;

    fn neg(self) -> Self::Output {
        match self {
            Value::NegInf => Value::Inf,
            Value::CentiPawn(val) => Value::CentiPawn(-val),
            Value::Inf => Value::NegInf,
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            // TODO
            Value::NegInf => write!(f, "mate {}", -1),
            Value::CentiPawn(val) => write!(f, "cp {}", val),
            Value::Inf => write!(f, "mate {}", 1),
        }
    }
}