use std::ops::Neg;

#[derive(Copy, Clone, PartialEq, PartialOrd)]
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