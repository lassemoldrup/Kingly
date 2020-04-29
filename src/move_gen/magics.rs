use crate::types::{Bitboard, Square, PieceType};
use crate::types::square_map::SquareMap;
use crate::lookup_table::Lookup;
use crate::move_gen::KNIGHT_MOVES;
use lazy_static::lazy_static;
use take_until::TakeUntilExt;
use crate::types::bitboard::RANK_8_BB;
use rand::rngs::SmallRng;
use std::iter::repeat;
use rand::{SeedableRng, Rng};

static ATTACK_TABLE: Lookup<[Bitboard; 107_648]> = Lookup::new([Bitboard::EMPTY; 107_648]);

#[derive(Copy, Clone)]
struct Magic {
    attacks: &'static [Bitboard],
    mask: Bitboard,
    factor: u64,
    shift: u8,
}

impl Magic {
    const fn new() -> Self {
        Magic {
            attacks: &[Bitboard::EMPTY; 0],
            mask: Bitboard::EMPTY,
            factor: 0u64,
            shift: 0u8,
        }
    }
}

static BISHOP_MAGICS: Lookup<SquareMap<Magic>> = Lookup::new(SquareMap::new(Magic::new()));
static ROOK_MAGICS: Lookup<SquareMap<Magic>> = Lookup::new(SquareMap::new(Magic::new()));

/// Generates a bitboard of all square attacks for a giving sliding piece
/// # Safety
/// `init_magics()` has to be called first
pub unsafe fn slider_attacks<const PT: PieceType>(occ: Bitboard, sq: Square) -> Bitboard {
    if PT == PieceType::Queen {
        slider_attacks::<{PieceType::Bishop}>(occ, sq) | slider_attacks::<{PieceType::Rook}>(occ, sq)
    } else {
        let magic = match PT {
            PieceType::Bishop => BISHOP_MAGICS.get(sq),
            PieceType::Rook => ROOK_MAGICS.get(sq),
            _ => panic!("slider_attacks called with illegal piece type"),
        };
        let key: usize = (((occ & magic.mask) * magic.factor) >> magic.shift as u64).into();
        magic.attacks[key]
    }
}

pub fn all_lookups_init() -> bool {
    ATTACK_TABLE.is_init()
        && BISHOP_MAGICS.is_init()
        && ROOK_MAGICS.is_init()
        && KNIGHT_MOVES.is_init()
        && KNIGHT_MOVES.is_init()
}

pub fn init_magics() {
    let mut next_idx = 0usize;
    fn init(kind: PieceType, next_idx: &mut usize, table: &Lookup<SquareMap<Magic>>) {
        for sq in Square::A1.range_to(Square::H8) {
            table.set(sq, find_magic(sq, kind, next_idx));
        }
        table.set_init();
    }
    init(PieceType::Bishop, &mut next_idx, &BISHOP_MAGICS);
    init(PieceType::Rook, &mut next_idx, &ROOK_MAGICS);
    ATTACK_TABLE.set_init();
}

/// Finds a magic factor for a given square for a bishop or rook
fn find_magic(sq: Square, kind: PieceType, next_idx: &mut usize) -> Magic {
    let mask: Bitboard = match kind {
        PieceType::Bishop => BISHOP_MASKS[sq],
        PieceType::Rook => ROOK_MASKS[sq],
        _ => panic!(),
    };
    let bits = mask.pop_count();
    let mut occupancies = [Bitboard::EMPTY; 4096];
    let mut attacks = [Bitboard::EMPTY; 4096];

    let array_len = 1 << bits as usize;
    for i in 0..array_len {
        occupancies[i] = index_to_occupancy(mask, i);
        attacks[i] = gen_attacks(sq, occupancies[i], kind);
    }

    let mut used = [Bitboard::EMPTY; 4096];
    // Todo: test seeds
    let mut rng = SmallRng::from_entropy();
    let factor = 'outer: loop {
        // Generate a random 64-bit number with few set bits
        let factor_guess = rng.gen::<u64>() & rng.gen::<u64>() & rng.gen::<u64>();
        // Set the first `array_len` elements of `used` to 0, aka. `Bitboard::EMPTY`
        unsafe { std::ptr::write_bytes(used.as_mut_ptr(), 0, array_len); }
        //used = [Bitboard::EMPTY; 4096];
        if ((mask * factor_guess) & RANK_8_BB).pop_count() < 6 {
            continue;
        }
        for (occ, att) in occupancies[0..array_len].iter().zip(attacks[0..array_len].iter()) {
            let magic_index: usize = ((*occ * factor_guess) >> (64 - bits)).into();
            if used[magic_index] == Bitboard::EMPTY {
                used[magic_index] = *att;
            } else if used[magic_index] != *att {
                continue 'outer;
            }
        }
        break factor_guess;
    };

    let end_idx = *next_idx + array_len;
    let attacks = ATTACK_TABLE.set_slice(*next_idx, end_idx, &used[0..array_len]);
    *next_idx = end_idx;

    let shift = 64 - bits as u8;
    Magic { attacks, mask, factor, shift }
}

lazy_static! {
    static ref BISHOP_MASKS: SquareMap<Bitboard> = get_masks(PieceType::Bishop);
    static ref ROOK_MASKS: SquareMap<Bitboard> = get_masks(PieceType::Rook);
}

fn get_masks(kind: PieceType) -> SquareMap<Bitboard> {
    let mut masks = SquareMap::new(Bitboard::EMPTY);
    for sq in Square::A1.range_to(Square::H8) {
        let mut result = Bitboard::EMPTY;

        let rank = sq.get_rank() as u8;
        let file = sq.get_file() as u8;
        if kind == PieceType::Bishop {
            for (r, f) in (rank + 1..7).zip(file + 1..7)
                                    .chain((1..rank).rev().zip(file + 1..7))
                                    .chain((1..rank).rev().zip((1..file).rev()))
                                    .chain((rank + 1..7).zip((1..file).rev())) {
                result.set(Square::get(f + 8 * r))
            }
        } else {
            for (r, f) in (rank + 1..7).zip(std::iter::repeat(file))
                                    .chain(std::iter::repeat(rank).zip(file + 1..7))
                                    .chain((1..rank).rev().zip(std::iter::repeat(file)))
                                    .chain(std::iter::repeat(rank).zip((1..file).rev())) {
                result.set(Square::get(f + 8 * r))
            }
        }

        masks[sq] = result
    }

    masks
}

fn index_to_occupancy(mask: Bitboard, index: usize) -> Bitboard {
    let mut occupancy = Bitboard::EMPTY;
    for (i, sq) in mask.iter().enumerate() {
        if (index >> i) & 1 == 1 {
            occupancy.set(sq);
        }
    }
    occupancy
}

fn gen_attacks(sq: Square, occ: Bitboard, kind: PieceType) -> Bitboard {
    let mut attacks = Bitboard::EMPTY;

    let rank = sq.get_rank() as u8;
    let file = sq.get_file() as u8;
    let square_occupied = |(r, f): &(u8, u8)| unsafe { occ.is_set(Square::from_unchecked(f + 8 * r)) };
    if kind == PieceType::Bishop {
        for (r, f) in (rank + 1..8).zip(file + 1..8).take_until(square_occupied)
                         .chain((0..rank).rev().zip(file + 1..8).take_until(square_occupied))
                         .chain((0..rank).rev().zip((0..file).rev()).take_until(square_occupied))
                         .chain((rank + 1..8).zip((0..file).rev()).take_until(square_occupied)) {
            unsafe { attacks.set(Square::from_unchecked(f + 8 * r)); }
        }
    } else {
        for (r, f) in (rank + 1..8).zip(repeat(file)).take_until(square_occupied)
                         .chain(repeat(rank).zip(file + 1..8).take_until(square_occupied))
                         .chain((0..rank).rev().zip(repeat(file)).take_until(square_occupied))
                         .chain(repeat(rank).zip((0..file).rev()).take_until(square_occupied)) {
            unsafe { attacks.set(Square::from_unchecked(f + 8 * r)); }
        }
    }

    attacks
}