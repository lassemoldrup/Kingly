# Kingly
Kingly is a UCI compliant chess engine written in Rust.

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
- PEXT sliding piece attacks
- Option to only generate captures

**Search**: Fail-Soft NegaMax-based AlphaBeta

**Search Enhancements**
- Iterative deepening
- Quiescence search
- Transposition table
- Aspiration windows
- Lazy SMP parallelization
- Null move pruning
- Check extensions
- Move reordering based on PV and MVV-LVA 

**Evaluation**: Naive piece count + mobility

**Other**
- Graphical search debugging tool

## Remaining tasks for the first version
These tasks will need to be completed before version 0.1.0 is ready.
- Extensive documentation
- More tests
- Proper benchmarking
- Better eval
- Endgame improvements
- Time management AI
- Magic bitboards fallback
