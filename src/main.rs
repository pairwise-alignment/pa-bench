mod params;
mod bench;
use crate::params::*;

use std::io;

use serde_json;

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input);
    let params = serde_json::from_str(&input);
    // TODO: run the correct algorithm based on params
}
