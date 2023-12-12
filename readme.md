<h1>
    <a href="https://docs.rs/crate/trace4rs/latest">
        <img
            src="https://github-production-user-asset-6210df.s3.amazonaws.com/7400284/289983235-f28e3d16-cdc4-4ab0-b157-22bd4adaca6b.png"
            alt="👁️ trace4rs"
            width=32
        />
    </a>
    trace4rs
</h1>

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

<a href="https://bencher.dev/perf/trace4rs?reports_per_page=4&reports_page=1&branches_per_page=8&branches_page=1&testbeds_per_page=8&testbeds_page=1&benchmarks_per_page=8&benchmarks_page=1"><img src="https://api.bencher.dev/v0/projects/trace4rs/perf/img?measures=6daa0563-984c-40c9-8716-cad463cc693b&branches=7c0ad2df-9b1c-4361-b0a5-8d87f8002dd4&testbeds=15a6cfb2-7ff5-4c89-abe9-d153f08a5ae0&benchmarks=422c918d-bf2f-4470-87b7-f06f6fc854ea%2C6d950af0-8d62-46fa-96e6-f694921e3cb6&title=log4rs+vs+trace4rs" title="log4rs vs trace4rs" alt="log4rs vs trace4rs for trace4rs - Bencher" /></a>

## Dev suggestions

- `cargo install cargo-binstall`
- `cargo install cargo-update`
- `cargo install cargo-nextest`
