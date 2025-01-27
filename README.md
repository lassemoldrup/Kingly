# Kingly

Kingly is a UCI compliant chess engine written in Rust.
You can play against it on [lichess](https://lichess.org/@/KinglyBot), where it is currently 2000-2100 rated.

## How to use

Install Rust through [rustup](https://rustup.rs/) and run with cargo:

```
cargo run --release
```

Note: For now, the version of rust required is not stabilised, so it might be necessary to switch to the beta branch with

```
rustup toolchain install beta
```

For a graphical experience, add Kingly to your favorite UCI compliant GUI, e.g. [Arena](http://www.playwitharena.de).

## Implemented Features

**Board representation**: Bitboards

**Move Generation**

- PEXT/PDEP based sliding piece attacks

**Search**: Fail-Soft Principal Variation Search

**Search Enhancements**

- Iterative deepening
- Quiescence search
- Transposition table
- Check extensions
- Move reordering based on PV and MVV-LVA
- Aspiration windows
- Null move pruning
- Reverse Futility Pruning

**Evaluation**: Piece/Square Tables

**Other**

- Graphical search debugging tool

## Coming Soon™

- Eval improvements
- Lazy SMP parallelization
- More search enhancements
- Magic Bitboard fallback for move generation
