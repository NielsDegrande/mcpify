use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(short, long, help = "Path to the OpenAPI JSON file")]
    pub file: PathBuf,

    #[arg(short, long, help = "Path to write the output directory")]
    pub output: PathBuf,
}
