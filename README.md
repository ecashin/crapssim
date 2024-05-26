# Craps Simulation

This simulator only knows how to bet with one strategy.
It makes a pass bet or come bet whenever it can,
and it always places odds bets when it can.

The motivation is to see how long play continues
under variations of play and how big the bankroll grows.
Each scenario runs until the player is broke.
Quantiles are printed.

    RUST_LOG=warn cargo run -- --n-trials 1000
    cargo run -- --help

It's a learning tool, so I wasn't avoiding panics
while programming.

