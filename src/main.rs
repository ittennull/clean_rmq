use clap::Parser;
use clean_rmq::Args;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    clean_rmq::run(args)
}
