# Pairwise Alignment Benchmarks

**Status: Work in progress**

This repository contains code to benchmark pairwise aligners.

Benchmarking is done using `job`s. Each job consists on an input dataset (a
`.seq` file), a cost model, and a tool with parameters. The `runner` binary runs one
job at a time and limits and measures the time and memory usage. The
`orchestrator` binary sets up multiple jobs using a `yaml` input file and calls the runner for each of them.

Results are incrementally accumulated in a `json` file.

## Crates

- `pa-bench-types`: Shared types.
- `runner`: Benchmarks a single job.
- `orchestrator`: Calls the runner for each job.

## Running the benchmark

1. Clone this repo and make sure you have Rust installed.
2. Build all crates in this repo with `cargo build --release`.
3. Run `cargo run --release` in the `evals` directory. See the [usage](#usage) below for details.

## Adding an aligner

The following files will need to be updated:

- `pa-bench-types/src/algorithms.rs`
- `runner/src/Cargo.toml`
- `runner/src/wrapper/mod.rs`

Then, the wrapper implementation for the aligner should be put into a new file
in `runner/src/wrapper/`. Remember to crash the program for unsupported parameter
configurations!

## Benchmarking features

**Settings**

- **Time limit**: Use `--time-limit 1h` to limit each run to `1` hour using `ulimit`.
- **Memory**: Use `--mem-limit GiB` to limit each run to `1GiB` of total memory using `ulimit`.
- **Nice**: Use `--nice=-20` to increase the priority of each runner job. This
  requires root. (See the end of this file.)
- **Parallel running**: Use `-j 10` to run `10` jobs in parallel. Each job (and
  the orchestrator) is pinned to a different core.
- **Incremental running**: With `--incremental`, jobs already present
  in the target `json` file are not rerun.
- **Force rerun**: With `--force-rerun`, the existing results file is ignored
  and datasets are regenerated.

**Output**

- Runtime of processing input pairs, excluding startup and file io time.
- Maximum memory usage, excluding the memory usage of the input data.
- Start and end time of job, for logging purposes.
- CPU frequency at start and end of job, as a sanity check.

## Input format

The input is specified as a `yaml` file containing:

- datasets: file paths or settings to generate datasets;
- traces: whether each tool computes a path or only the edit distance;
- costs: the cost models to run all aligners on;
- algos: the algorithms (aligners with parameters) to use.

A job is created for the each combination of the 4 lists.

Example:

```yaml
datasets:
  # Hardcoded data
  - !Data
    - - CGCTGGCTGCTGCCACTAACTCCGTATAGTCTCACCAAGT
      - CGCTGGCTCGCCTGCCACGTAACTCCGTATAGTCTCACCAACTGTCAGTT
    - - AACCAGGGTACACCGACTAATCCACGCACAAGTTGGGGTC
      - ACAGGTACACCACTATCACGACAAGTTGGGTC
  # Path to a single .seq file.
  - !File path/to/sequences.seq
  # Recursively finds all non-hidden .seq files in a directory.
  - !Directory path/to/directory
  # Download and extract a zip file containing .seq files.
  - !Download
    url: https://github.com/RagnarGrootKoerkamp/astar-pairwise-aligner/releases/download/datasets/chm13.v1.1-ont-ul.500kbps.zip
    dir: human/chm13/
  # Generated data
  - !Generated # Seed for the RNG.
    seed: 31415
    # The approximate total length of the input sequences.
    total_size: 100000
    # The error models to use. See pa-generate crate for more info:
    # https://github.com/pairwise-alignment/pa-generate
    error_models:
      - Uniform
      # - NoisyInsert
      # - NoisyDelete
      # - NoisyMove
      # - NoisyDuplicate
      # - SymmetricRepeat
    error_rates: [0.01, 0.05, 0.1, 0.1]
    lengths: [100, 1000, 10000, 100000]
# Run both with and without traces
traces: [false, true]
costs:
  # unit costs
  - { sub: 1, open: 0, extend: 1 }
  # affine costs
  - { sub: 1, open: 1, extend: 1 }
  - { sub: 2, open: 3, extend: 2 }
algos:
  - !BlockAligner
    size: !Size [32, 8192]
  - !ParasailStriped
  - !Edlib
  - !TripleAccel
  - !Wfa
    memory_model: !MemoryUltraLow
    heuristic: !None
  - !Ksw2
    method: !GlobalSuzukiSse
    band_doubling: false
  - !Ksw2
    method: !GlobalSuzukiSse
    band_doubling: true
```

## Usage

Minimal usage to get started: first build the `runner` in release mode
and then run the orchestrator in the root directory with `-q` (i.e. `5` parallel
jobs and reusing previous results) on one or more experiment files:

```sh
cargo build --release
cargo run --release -- -q evals/experiments/test.yaml
```

This writes incremental results to
`evals/results/test.json` (this includes jobs that are not part of the
experiment anymore) and the exact jobs listed in the current experiment
are written to `evals/results/test.exact.json`.

Succinct help (run with `--help` for more):

```text
A binary to run and benchmark multiple pairwise alignment tasks.

Usage: orchestrator [OPTIONS] [EXPERIMENTS]...

Arguments:
  [EXPERIMENTS]...  Path to an experiment yaml file

Options:
  -o, --output <OUTPUT>  Path to the output json file. By default mirrors the `experiments` dir in `results`
  -j <NUM_JOBS>          Number of parallel jobs to use
  -i, --incremental      Skip jobs already present in the results file
  -r, --rerun-failed     In combination with --incremental, also rerun failed jobs
  -q, --quick            Shorthand for '-j5 --incremental --rerun-failed'
      --release          Shorthand for '-j1 --nice=-20'
  -h, --help             Print help (see more with '--help')

Limits:
  -t, --time-limit <TIME_LIMIT>  Time limit. Defaults to value in experiment yaml or 1m
  -m, --mem-limit <MEM_LIMIT>    Memory limit. Defaults to value in experiment yaml or 1GiB
      --nice <NICE>              Process niceness. '--nice=-20' for highest priority

Output:
  -v, --verbose  Print jobs started and finished
      --stderr   Show stderr of runner process
```

## Notes on benchmarking

**Niceness.**
Changing niceness to `-20` (the highest priority) requires running the
orchestrator as root. Alternatively, you could add the following line to
`/etc/security/limits.conf` to allow your user to use lower niceness values:

```text
<username> - nice -20
```
