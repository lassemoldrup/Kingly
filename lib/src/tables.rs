use std::alloc::{self, Layout};
use std::convert::TryFrom;
use std::marker::PhantomPinned;
use std::pin::Pin;
use std::ptr::NonNull;

use bitintr::{Pdep, Pext};
use itertools::Itertools;
use lazy_static::lazy_static;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;

use crate::collections::SquareMap;
use crate::types::{Bitboard, BoardVector, Color, File, Piece, PieceKind, Rank, Square};
use crate::{bb, Position};

lazy_static! {
    static ref TABLES: Tables = Tables::init();
}

pub struct Tables {
    pub white_pawn_attacks: SquareMap<Bitboard>,
    pub black_pawn_attacks: SquareMap<Bitboard>,
    pub knight_attacks: SquareMap<Bitboard>,
    pub king_attacks: SquareMap<Bitboard>,
    /// Relevant occupancy squares for bishop attacks.
    pub bishop_masks: SquareMap<Bitboard>,
    /// Relevant occupancy squares for rook attacks.
    pub rook_masks: SquareMap<Bitboard>,
    pub slider_attacks: SliderAttacks,
    /// The entire line between two squares, empty bb if not on line.
    pub line_through: SquareMap<SquareMap<Bitboard>>,
    /// All the squares between two squares, includes the destination square, empty bb if not on a line
    pub ray_to: SquareMap<SquareMap<Bitboard>>,
    pub zobrist_randoms: ZobristRandoms,
}

impl Tables {
    pub fn get() -> &'static Self {
        &TABLES
    }

    fn init() -> Self {
        let bishop_masks = Self::init_bishop_masks();
        let rook_masks = Self::init_rook_masks();
        Self {
            white_pawn_attacks: Self::init_white_pawn_attacks(),
            black_pawn_attacks: Self::init_black_pawn_attacks(),
            knight_attacks: Self::init_knight_attacks(),
            king_attacks: Self::init_king_attacks(),
            bishop_masks,
            rook_masks,
            slider_attacks: SliderAttacks::init(&bishop_masks, &rook_masks),
            line_through: Self::init_line_through(),
            ray_to: Self::init_ray_to(),
            zobrist_randoms: ZobristRandoms::init(),
        }
    }

    fn init_white_pawn_attacks() -> SquareMap<Bitboard> {
        let move_vecs = [BoardVector::NORTH_WEST, BoardVector::NORTH_EAST];
        Self::init_step_attacks(&move_vecs)
    }

    fn init_black_pawn_attacks() -> SquareMap<Bitboard> {
        let move_vecs = [BoardVector::SOUTH_WEST, BoardVector::SOUTH_EAST];
        Self::init_step_attacks(&move_vecs)
    }

    fn init_knight_attacks() -> SquareMap<Bitboard> {
        let move_vecs = [
            BoardVector::NORTH + BoardVector::NORTH_WEST,
            BoardVector::NORTH + BoardVector::NORTH_EAST,
            BoardVector::EAST + BoardVector::NORTH_EAST,
            BoardVector::EAST + BoardVector::SOUTH_EAST,
            BoardVector::SOUTH + BoardVector::SOUTH_EAST,
            BoardVector::SOUTH + BoardVector::SOUTH_WEST,
            BoardVector::WEST + BoardVector::SOUTH_WEST,
            BoardVector::WEST + BoardVector::NORTH_WEST,
        ];
        Self::init_step_attacks(&move_vecs)
    }

    fn init_king_attacks() -> SquareMap<Bitboard> {
        let move_vecs = [
            BoardVector::NORTH,
            BoardVector::NORTH_EAST,
            BoardVector::EAST,
            BoardVector::SOUTH_EAST,
            BoardVector::SOUTH,
            BoardVector::SOUTH_WEST,
            BoardVector::WEST,
            BoardVector::NORTH_WEST,
        ];
        Self::init_step_attacks(&move_vecs)
    }

    fn init_step_attacks(move_vecs: &[BoardVector]) -> SquareMap<Bitboard> {
        SquareMap::from_fn(|sq| {
            move_vecs
                .iter()
                .filter_map(|&vec| sq.add_checked(vec))
                .collect()
        })
    }
    fn init_bishop_masks() -> SquareMap<Bitboard> {
        SquareMap::from_fn(|sq| {
            gen_bishop_attacks_slow(sq, bb!())
                - Bitboard::from(Rank::First)
                - Bitboard::from(Rank::Eighth)
                - Bitboard::from(File::A)
                - Bitboard::from(File::H)
        })
    }

    fn init_rook_masks() -> SquareMap<Bitboard> {
        SquareMap::from_fn(|sq| {
            gen_rook_attacks_slow(sq, bb!())
                - bb!(
                    Square::from_rank_file(Rank::First, sq.file()),
                    Square::from_rank_file(Rank::Eighth, sq.file()),
                    Square::from_rank_file(sq.rank(), File::A),
                    Square::from_rank_file(sq.rank(), File::H),
                )
        })
    }

    fn init_line_through() -> SquareMap<SquareMap<Bitboard>> {
        SquareMap::from_fn(|from| {
            let from_rank = from.rank() as i8;
            let from_file = from.file() as i8;

            SquareMap::from_fn(|to| {
                if from == to {
                    return bb!();
                }

                let to_rank = to.rank() as i8;
                let to_file = to.file() as i8;
                let delta_rank = to_rank - from_rank;
                let delta_file = to_file - from_file;

                if from_rank == to_rank {
                    Bitboard::RANKS[from_rank as usize]
                } else if from_file == to_file {
                    Bitboard::FILES[from_file as usize]
                } else if delta_rank == delta_file {
                    let diag_idx = (7 + from_rank - from_file) as usize;
                    Bitboard::DIAGS[diag_idx]
                } else if delta_rank == -delta_file {
                    let anit_diag_idx = (from_rank + from_file) as usize;
                    Bitboard::ANTI_DIAGS[anit_diag_idx]
                } else {
                    bb!()
                }
            })
        })
    }

    fn init_ray_to() -> SquareMap<SquareMap<Bitboard>> {
        SquareMap::from_fn(|from| {
            let from_rank = from.rank() as i8;
            let from_file = from.file() as i8;

            SquareMap::from_fn(|to| {
                if from == to {
                    return bb!(to);
                }

                let to_rank = to.rank() as i8;
                let to_file = to.file() as i8;
                let delta_rank = to_rank - from_rank;
                let delta_file = to_file - from_file;

                if delta_rank.abs() != delta_file.abs() && delta_rank != 0 && delta_file != 0 {
                    return bb!();
                }

                let rank_step = delta_rank.signum();
                let file_step = delta_file.signum();

                let mut rank = from_rank + rank_step;
                let mut file = from_file + file_step;
                let mut res = bb!();
                while rank != to_rank || file != to_file {
                    let sq = Square::try_from((8 * rank + file) as u8)
                        .expect("rank and file should be in [0; 7]");
                    res.add_sq(sq);
                    rank += rank_step;
                    file += file_step;
                }
                res.with_sq(to)
            })
        })
    }

    pub fn gen_bishop_attacks(&self, occ: Bitboard, sq: Square) -> Bitboard {
        let bishop_attacks = self.slider_attacks.bishop_attacks(sq);
        let occ: u64 = occ.into();
        let key = occ.pext(self.bishop_masks[sq].into()) as usize;
        unsafe { *bishop_attacks.get_unchecked(key) }
    }

    pub fn gen_rook_attacks(&self, occ: Bitboard, sq: Square) -> Bitboard {
        let rook_attacks = self.slider_attacks.rook_attacks(sq);
        let occ: u64 = occ.into();
        let key = occ.pext(self.rook_masks[sq].into()) as usize;
        unsafe { *rook_attacks.get_unchecked(key) }
    }

    pub fn gen_attacks_from_sq(&self, occ: Bitboard, pce: Piece, sq: Square) -> Bitboard {
        match pce.kind() {
            PieceKind::Pawn => match pce.color() {
                Color::White => self.white_pawn_attacks[sq],
                Color::Black => self.black_pawn_attacks[sq],
            },
            PieceKind::Knight => self.knight_attacks[sq],
            PieceKind::Bishop => self.gen_bishop_attacks(occ, sq),
            PieceKind::Rook => self.gen_rook_attacks(occ, sq),
            PieceKind::Queen => self.gen_bishop_attacks(occ, sq) | self.gen_rook_attacks(occ, sq),
            PieceKind::King => self.king_attacks[sq],
        }
    }

    pub fn gen_attacks(&self, pce_bb: Bitboard, occ: Bitboard, pce: Piece) -> Bitboard {
        match pce.kind() {
            PieceKind::Pawn => {
                let (left, right, first_file, last_file) = match pce.color() {
                    Color::White => (
                        BoardVector::NORTH_WEST,
                        BoardVector::NORTH_EAST,
                        Bitboard::from(File::A),
                        Bitboard::from(File::H),
                    ),
                    Color::Black => (
                        BoardVector::SOUTH_EAST,
                        BoardVector::SOUTH_WEST,
                        Bitboard::from(File::H),
                        Bitboard::from(File::A),
                    ),
                };
                ((pce_bb >> left) - last_file) | ((pce_bb >> right) - first_file)
            }
            _ => pce_bb.into_iter().fold(bb!(), |atks, sq| {
                atks | self.gen_attacks_from_sq(occ, pce, sq)
            }),
        }
    }

    pub fn get_mobility(&self, position: &Position, color: Color) -> usize {
        use PieceKind::*;

        let occ = position.pieces.occupied();
        [Pawn, Knight, Bishop, Rook, Queen, King]
            .into_iter()
            .map(|kind| {
                let pce = Piece(kind, color);
                let pce_bb = position.pieces.get_bb(pce);
                self.gen_attacks(pce_bb, occ, pce).len()
            })
            .sum()
    }
}

pub struct SliderAttacks {
    _tables: Pin<Box<SliderAttacksTables>>,
    /// Points to the slice of `SliderAttackTables` that holds bishop attacks for a given square.
    bishop_attacks: SquareMap<NonNull<[Bitboard]>>,
    /// Points to the slice of `SliderAttackTables` that holds rook attacks for a given square.
    rook_attacks: SquareMap<NonNull<[Bitboard]>>,
}

impl SliderAttacks {
    fn init(bishop_masks: &SquareMap<Bitboard>, rook_masks: &SquareMap<Bitboard>) -> Self {
        // We do not want to allocate the big arrays on the stack, so we allocate them on the heap
        let layout = Layout::new::<SliderAttacksTables>();
        // Safety: The layout is correct
        let ptr = unsafe { alloc::alloc(layout) as *mut SliderAttacksTables };

        let mut bishop_attacks = SquareMap::new([NonNull::from(&[][..]); 64]);
        let mut rook_attacks = SquareMap::new([NonNull::from(&[][..]); 64]);
        let mut num_bishop_init = 0;
        let mut num_rook_init = 0;
        for sq in Square::iter() {
            let count = 1 << bishop_masks[sq].len();
            // Iterate over all possible occupancies
            for key in 0..count as u64 {
                // Place the occupancy bits of `key` on the squares that are relevant for a bishop on `sq`
                let occ_bb = key.pdep(bishop_masks[sq].into()).into();
                let atk_bb = gen_bishop_attacks_slow(sq, occ_bb);
                let idx = num_bishop_init + key as usize;
                // Safety: The pointer is valid
                unsafe { (*ptr).bishop[idx] = atk_bb };
            }
            // Safety: The pointer is valid
            bishop_attacks[sq] =
                unsafe { (*ptr).bishop[num_bishop_init..num_bishop_init + count as usize].into() };
            num_bishop_init += count;

            let count = 1 << rook_masks[sq].len();
            // Iterate over all possible occupancies
            for key in 0..count as u64 {
                // Place the occupancy bits of `key` on the squares that are relevant for a rook on `sq`
                let occ_bb = key.pdep(rook_masks[sq].into()).into();
                let atk_bb = gen_rook_attacks_slow(sq, occ_bb);
                let idx = num_rook_init + key as usize;
                // Safety: The pointer is valid
                unsafe { (*ptr).rook[idx] = atk_bb };
            }
            // Safety: The pointer is valid
            rook_attacks[sq] =
                unsafe { (*ptr).rook[num_rook_init..num_rook_init + count as usize].into() };
            num_rook_init += count;
        }
        assert_eq!(num_bishop_init, NUM_BISHOP_ATTACKS);
        assert_eq!(num_rook_init, NUM_ROOK_ATTACKS);
        // Safety: The pointer is valid
        unsafe {
            // Probably not necessary, but just to be sure
            (*ptr)._pin = PhantomPinned;
        }

        // Safety: We have initialized all fields of the struct
        let _tables = Box::into_pin(unsafe { Box::from_raw(ptr) });
        Self {
            _tables,
            bishop_attacks,
            rook_attacks,
        }
    }

    pub fn bishop_attacks(&self, sq: Square) -> &[Bitboard] {
        // Safety: The bishop_attacks field is initialized correctly to point to the correct slice
        unsafe { self.bishop_attacks[sq].as_ref() }
    }

    pub fn rook_attacks(&self, sq: Square) -> &[Bitboard] {
        // Safety: The rook_attacks field is initialized correctly to point to the correct slice
        unsafe { self.rook_attacks[sq].as_ref() }
    }
}

// Safety: The API only allows for read-only access to the tables
unsafe impl Sync for SliderAttacks {}

const NUM_BISHOP_ATTACKS: usize = 4 * (1 << 9) + 12 * (1 << 7) + 44 * (1 << 5) + 4 * (1 << 6);
const NUM_ROOK_ATTACKS: usize = 4 * (1 << 12) + 24 * (1 << 11) + 36 * (1 << 10);

struct SliderAttacksTables {
    bishop: [Bitboard; NUM_BISHOP_ATTACKS],
    rook: [Bitboard; NUM_ROOK_ATTACKS],
    _pin: PhantomPinned,
}

pub struct ZobristRandoms {
    pub pieces: [[u64; 64]; 12],
    pub to_move: u64,
    pub castling: [u64; 16],
    pub en_passant: [u64; 8],
}

impl ZobristRandoms {
    fn init() -> Self {
        // TODO: Test different seeds
        let mut rng = ChaCha20Rng::from_seed([25; 32]);
        Self {
            pieces: rng.gen(),
            to_move: rng.gen(),
            castling: rng.gen(),
            en_passant: rng.gen(),
        }
    }
}

fn gen_bishop_attacks_slow(sq: Square, occ: Bitboard) -> Bitboard {
    let sq_not_occ = |sq: &Square| !occ.contains(*sq);

    let rank = sq.rank();
    let file = sq.file();
    let north_east = rank
        .iter_after()
        .zip(file.iter_after())
        .map(|(r, f)| Square::from_rank_file(r, f))
        .take_while_inclusive(sq_not_occ);
    let south_east = rank
        .iter_before()
        .zip(file.iter_after())
        .map(|(r, f)| Square::from_rank_file(r, f))
        .take_while_inclusive(sq_not_occ);
    let south_west = rank
        .iter_before()
        .zip(file.iter_before())
        .map(|(r, f)| Square::from_rank_file(r, f))
        .take_while_inclusive(sq_not_occ);
    let north_west = rank
        .iter_after()
        .zip(file.iter_before())
        .map(|(r, f)| Square::from_rank_file(r, f))
        .take_while_inclusive(sq_not_occ);

    north_east
        .chain(south_east)
        .chain(south_west)
        .chain(north_west)
        .collect()
}

fn gen_rook_attacks_slow(sq: Square, occ: Bitboard) -> Bitboard {
    let sq_not_occ = |sq: &Square| !occ.contains(*sq);

    let rank = sq.rank();
    let file = sq.file();
    let north = rank
        .iter_after()
        .map(|r| Square::from_rank_file(r, file))
        .take_while_inclusive(sq_not_occ);
    let east = file
        .iter_after()
        .map(|f| Square::from_rank_file(rank, f))
        .take_while_inclusive(sq_not_occ);
    let south = rank
        .iter_before()
        .map(|r| Square::from_rank_file(r, file))
        .take_while_inclusive(sq_not_occ);
    let west = file
        .iter_before()
        .map(|f| Square::from_rank_file(rank, f))
        .take_while_inclusive(sq_not_occ);

    north.chain(east).chain(south).chain(west).collect()
}

#[cfg(test)]
mod tests {
    use crate::bb;
    use crate::types::Square;

    use super::Tables;

    #[test]
    fn bishop_masks_initialized_correctly() {
        let tables = Tables::get();

        use Square::*;
        assert_eq!(tables.bishop_masks[A8], bb!(B7, C6, D5, E4, F3, G2));
        assert_eq!(tables.bishop_masks[B2], bb!(C3, D4, E5, F6, G7));
        assert_eq!(
            tables.bishop_masks[D5],
            bb!(E6, F7, E4, F3, G2, C4, B3, C6, B7)
        );
    }

    #[test]
    fn rook_masks_initialized_correctly() {
        let tables = Tables::get();

        use Square::*;
        assert_eq!(
            tables.rook_masks[A8],
            bb!(B8, C8, D8, E8, F8, G8, A7, A6, A5, A4, A3, A2)
        );
        assert_eq!(
            tables.rook_masks[B2],
            bb!(B3, B4, B5, B6, B7, C2, D2, E2, F2, G2)
        );
        assert_eq!(
            tables.rook_masks[D5],
            bb!(E5, F5, G5, D4, D3, D2, C5, B5, D6, D7)
        );
    }

    #[test]
    fn line_through_initialized_correctly() {
        let tables = Tables::get();

        use Square::*;
        assert_eq!(
            tables.line_through[B1][B5],
            bb!(B1, B2, B3, B4, B5, B6, B7, B8)
        );
        assert_eq!(tables.line_through[F8][C5], bb!(A3, B4, C5, D6, E7, F8));
        assert_eq!(
            tables.line_through[D4][E4],
            bb!(A4, B4, C4, D4, E4, F4, G4, H4)
        );
        assert_eq!(
            tables.line_through[A8][H1],
            bb!(A8, B7, C6, D5, E4, F3, G2, H1)
        );
        assert_eq!(tables.line_through[C4][D6], bb!());
    }

    #[test]
    fn ray_to_initialized_correctly() {
        let tables = Tables::get();

        use Square::*;
        assert_eq!(tables.ray_to[B1][B5], bb!(B2, B3, B4, B5));
        assert_eq!(tables.ray_to[F8][C5], bb!(E7, D6, C5));
        assert_eq!(tables.ray_to[C4][D6], bb!());
    }
}
