use clap::Parser;
use lamprey_cli::args::ly::Args;

fn main() {
    let args = Args::parse();
    dbg!(args);
}
