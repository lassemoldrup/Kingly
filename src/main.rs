extern crate crusty;

use crusty::move_gen::{init_magics, init_non_sliders, generate_all};
use crusty::position::Position;
use arrayvec::ArrayVec;
use crusty::types::{Move, Color};

fn main() {
    init_magics();
    init_non_sliders();
    let pos = Position::new_default();
    let mut moves: ArrayVec<[Move; 256]> = ArrayVec::new();
    unsafe { generate_all::<{Color::White}>(&pos, &mut moves); }
    print_moves(&moves);
}

fn print_moves(moves: &ArrayVec<[Move; 256]>) {
    print!("[ ");
    for m in moves {
        print!("{}, ", m)
    }
    println!("]");
}