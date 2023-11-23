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


## Todo

- Need to refactor up the reload stuff so that we can bubble up the ExtLyr stuff high enough to make it useful
- [ ] Allow for generic "Layers" which can be plugged from tracing ecosystem
  - We need to balance this with also allowing for no heap allocation of the span-tree layer
  - The reason for this is that we have parts of the ecosystem that don't use FormatEvent, we will therefore need to come up with a mechanism to use them which is outside that.
