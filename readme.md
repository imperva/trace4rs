# trace4rs

This crate allows users to configure output from
[`tracing`](docs.rs/tracing) in the same way as you would configure the
output of [`log4rs`](docs.rs/log4rs).

## Overview

For a usage example see the `examples` folder or `src/test.rs`.

### Benchmarks & Results

The takeaway is that the actual appenders are roughly equivalent in
performance. However, when using the `tracing` macros vs the `log` macros
the appender performance is roughly 2 orders of magnitude larger.
See for yourself with `cargo bench --features tracing-macros`

## Dev suggestions

- `cargo install cargo-binstall`
- `cargo install cargo-nextest`

## Todo

- verify cspell working
- add an example for metrics functionality in lieu of a pub method
- remove custom_test_frameworks usage
