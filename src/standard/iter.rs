use crate::framework::square::Square;

pub struct BitboardIter;

impl Iterator for BitboardIter {
    type Item = Square;

    fn next(&mut self) -> Option<Self::Item> {
        unimplemented!()
    }
}