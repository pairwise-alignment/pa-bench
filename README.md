# Pairwise Alignment Benchmarks

**Status: Work in progress**

This repository contains code to benchmark pairwise aligners.

Benchmarking is done using `job`s. Each job consists on an input dataset (a
`.seq` file), a cost model, and a tool with parameters. The `runner` binary runs one
job at a time and limits and measures the time and memory usage. The
`orchestrator` binary sets up multiple jobs and calls the runner for each of them.

## Crates

- `pa-bench-types`: Shared types.
- `runner`: Benchmarks a single job.
- `orchestrator`: Calls the runner for each job.

## Running the benchmark

1. Clone this repo and make sure you have Rust installed.
2. Build all crates in this repo with `cargo build --release`.
3. `cd orchestrator && cargo run --release`.
