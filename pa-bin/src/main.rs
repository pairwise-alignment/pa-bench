use bio::io::fasta;
use clap::{value_parser, Args, Parser};
use itertools::Itertools;
use pa_types::{CostModel, Seq};
use pa_wrapper::Aligner;
use std::fs::{self, File};
use std::io::{BufRead, BufWriter, Write};
use std::path::PathBuf;
use std::process::exit;

/// CLI tool that wraps other aligners and runs them on the given input.
#[derive(Parser)]
#[command(author)]
struct Cli {
    /// Return only cost (no traceback).
    #[clap(long)]
    cost_only: bool,

    /// Do not print anything to stderr.
    #[clap(long)]
    silent: bool,

    /// (Directory of) .seq, .txt, or Fasta files with sequence pairs to align.
    ///
    /// For directories, this is not recursive. Only files in the directory itself are processed.
    #[clap(value_parser = value_parser!(PathBuf), display_order = 1, required_unless_present = "print_params")]
    input: Option<PathBuf>,

    /// Write a .csv of `{cost},{cigar}` lines. Defaults to input file with .csv extension.
    ///
    /// If input is a file, output is written to this file. If input is a directory, output is
    /// written to a file in this directory with the same name as the input file.
    #[clap(value_parser = value_parser!(PathBuf), display_order = 1)]
    output: Option<PathBuf>,

    #[clap(flatten, next_help_heading = "Aligner")]
    aligner: AlignerArgs,

    /// The parameters are json instead of yaml.
    #[clap(long)]
    json: bool,

    /// Whether to return a traceback.
    #[clap(flatten, next_help_heading = "Cost model")]
    cost_model: CostModel,
}

#[derive(Args)]
#[group(required = true, multiple = false)]
struct AlignerArgs {
    /// The aligner to use with default parameters.
    #[clap(long, value_name = "ALIGNER")]
    aligner: Option<Aligner>,

    /// Yaml/json string of aligner parameters.
    #[clap(long, value_name = "STRING")]
    params: Option<String>,

    /// File with aligner parameters.
    #[clap(
        long,
        conflicts_with = "params",
        conflicts_with = "aligner",
        value_name = "PATH"
    )]
    params_file: Option<PathBuf>,

    /// Print default parameters for the given aligner.
    #[clap(long, value_name = "ALIGNER")]
    print_params: Option<Aligner>,
}

fn main() {
    let args = Cli::parse();

    // Exactly one of these will be true because of the AlignerArgs group.
    let aligner_params = if let Some(aligner) = args.aligner.aligner {
        aligner.default_params()
    } else if let Some(params) = args.aligner.params {
        if args.json {
            serde_json::from_str(&params).expect("Failed to parse parameters as json")
        } else {
            serde_yaml::from_str(&params).expect("Failed to parse parameters as yaml")
        }
    } else if let Some(params_file) = args.aligner.params_file {
        let params = fs::read_to_string(params_file).unwrap();
        if args.json {
            serde_json::from_str(&params).expect("Failed to parse parameters as json")
        } else {
            serde_yaml::from_str(&params).expect("Failed to parse parameters as yaml")
        }
    } else if let Some(aligner) = args.aligner.print_params {
        let params = aligner.default_params();
        if args.json {
            println!("{}", serde_json::to_string_pretty(&params).unwrap());
        } else {
            println!("{}", serde_yaml::to_string(&params).unwrap());
        }
        exit(0);
    } else {
        unreachable!()
    };

    let Some(input) = args.input else {
        eprintln!("Input is required");
        exit(1);
    };

    let mut aligner = aligner_params
        .build_aligner(args.cost_model, !args.cost_only, 0)
        .0;
    // Parse file
    let files = if input.is_file() {
        let output = args.output.unwrap_or(input.with_extension("csv"));
        vec![(input.clone(), output)]
    } else {
        if let Some(output) = args.output.as_ref() {
            if output.exists() && !output.is_dir() {
                eprintln!("Output must be a directory if input is a directory");
                exit(1);
            }
            if !output.exists() {
                eprintln!("Creating output directory {}", output.display());
                fs::create_dir_all(&output).unwrap();
            }
        }
        input
            .read_dir()
            .expect(&format!("{} is not a file or directory", input.display()))
            .filter_map(|x| {
                let x = x.unwrap();
                if !x.file_type().unwrap().is_file() {
                    return None;
                }
                if !["fna", "fa", "fasta", "seq", "txt"]
                    .contains(&x.path().extension().unwrap_or_default().to_str().unwrap())
                {
                    return None;
                }
                let input = x.path();
                let output = match args.output.clone() {
                    Some(o) => o.join(input.with_extension("csv").file_name().unwrap()),
                    None => input.with_extension("csv"),
                };

                Some((input, output))
            })
            .collect()
    };

    let mut done = 0;
    for (i, o) in files {
        let header = format!("{} => {}", i.display(), o.display());

        // Process the input.
        let mut run_pair = |a: Seq, b: Seq, o: &mut BufWriter<File>| {
            let (cost, cigar, _stats) = aligner.align(a, b);

            done += 1;
            if !args.silent {
                eprint!("\rDone {done:>6}: {header}",);
            }

            writeln!(
                o,
                "{cost},{}",
                cigar.map_or(String::new(), |c| c.to_string())
            )
            .unwrap();
        };

        let ext = i.extension().expect("Unknown file extension");
        let i = std::io::BufReader::new(std::fs::File::open(&i).unwrap());
        let mut o = std::io::BufWriter::new(std::fs::File::create(o).unwrap());
        match ext {
            ext if ext == "seq" || ext == "txt" => {
                for (a, b) in i.lines().map(|l| l.unwrap().into_bytes()).tuples() {
                    let mut a = &a[..];
                    let mut b = &b[..];
                    if ext == "seq" {
                        let (&a0, ar) = a.split_first().unwrap();
                        a = ar;
                        assert_eq!(
                            a0 as char, '>',
                            "In a .seq file, lines must alternating start with '>' and '<'."
                        );
                        let (&b0, br) = b.split_first().unwrap();
                        b = br;
                        assert_eq!(
                            b0 as char, '<',
                            "In a .seq file, lines must alternating start with '>' and '<'."
                        );
                    }
                    run_pair(&a, &b, &mut o);
                }
            }
            ext if ext == "fna" || ext == "fa" || ext == "fasta" => {
                for (a, b) in fasta::Reader::new(i).records().tuples() {
                    run_pair(a.unwrap().seq(), b.unwrap().seq(), &mut o);
                }
            }
            ext => {
                unreachable!("Unknown file extension {ext:?}. Must be in {{seq,txt,fna,fa,fasta}}.")
            }
        };
    }

    eprintln!();
}
