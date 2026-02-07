use clap::{Parser, Subcommand};
use std::path::PathBuf;

use pyra_compiler::{compile_file_to_abi_and_bin, compile_file, GasReport};
use pyra_compiler::ir::lower_program;
use pyra_compiler::{harden, add_reentrancy_guard, StorageLayout};

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
        #[arg(long = "gas-report")]
        gas_report: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Build { input, out_dir, gas_report } => {
            match compile_file_to_abi_and_bin(&input, out_dir.as_deref()) {
                Ok(_) => {
                    if gas_report {
                        if let Ok(program) = compile_file(&input) {
                            let mut module = lower_program(&program);
                            harden(&mut module);
                            let layout = StorageLayout::from_program(&program);
                            add_reentrancy_guard(&mut module, layout.slot_count());
                            let report = GasReport::from_module(&module);
                            println!("Gas Report");
                            println!("{}", "=".repeat(50));
                            for f in &report.functions {
                                println!(
                                    "  {} (0x{})  ~{} gas",
                                    f.name,
                                    hex::encode(f.selector),
                                    f.estimated_gas
                                );
                            }
                            println!("  constructor            ~{} gas", report.constructor_gas);
                            println!("  dispatch overhead      ~{} gas", report.dispatch_overhead);
                        }
                    }
                    std::process::exit(0)
                }
                Err(err) => {
                    eprintln!("{err}");
                    std::process::exit(1)
                }
            }
        }
    }
}
