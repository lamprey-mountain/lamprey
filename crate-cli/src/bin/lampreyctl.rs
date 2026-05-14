use clap::Parser;
use lamprey_cli::args::lampreyctl::Args;

fn main() {
    let args = Args::parse();
    dbg!(args);
}
