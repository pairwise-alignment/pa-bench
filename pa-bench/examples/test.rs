use clap::Parser;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
enum E {
    A,
    B,
}

#[derive(Serialize, Deserialize)]
struct Inner {
    e: E,
}

#[derive(Serialize, Deserialize)]
struct Outer {
    #[serde(flatten)]
    inner: Inner,
}

#[derive(Parser, Debug)]
struct Opts {
    #[clap(long, action = clap::ArgAction::Set, default_value = "true")]
    a: bool,
}

fn main() {
    let args = Opts::parse();
    println!("{args:?}");

    let yaml = "e: !A";

    // Works fine:
    let _inner: Inner = serde_yaml::from_str(&yaml).unwrap();

    // Fails at runtime with:
    // e: untagged and internally tagged enums do not support enum input
    let _outer: Outer = serde_yaml::from_str(&yaml).unwrap();
}
