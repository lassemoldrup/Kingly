use arrayvec::ArrayVec;
use crate::types::{Move, Color};
use crate::move_gen::generate_all;
use crate::position::Position;

pub fn perft(pos: &mut Position, depth: u32) -> u64 {
    let mut moves = ArrayVec::<[Move;256]>::new();

    if depth == 0 {
        return 1;
    }

    let mut nodes = 0u64;
    if pos.get_to_move() == Color::White {
        unsafe { generate_all::<{Color::White}>(pos, &mut moves); }
    } else {
        unsafe { generate_all::<{Color::Black}>(pos, &mut moves); }
    }
    for m in moves {
        pos.make_move(m);
        nodes += perft(pos, depth - 1);
        pos.unmake_move(m)
    }
    nodes
}