use clap::{Parser, Subcommand};
use std::path::PathBuf;

use pyra_compiler::compile_file_to_abi_and_bin;

#[derive(Parser)]
#[command(name = "pyra", version, about = "Pyra compiler")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Build {
        input: PathBuf,
        #[arg(short = 'o', long = "out-dir")]
        out_dir: Option<PathBuf>,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Build { input, out_dir } => match compile_file_to_abi_and_bin(&input, out_dir.as_deref()) {
            Ok(_) => std::process::exit(0),
            Err(err) => {
                eprintln!("{err}");
                std::process::exit(1)
            }
        },
    }
}
