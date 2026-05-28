mod cmd;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "lin", version, about = "The Lin language compiler and toolchain")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile a .lin source file to a native binary
    Build(cmd::build::BuildArgs),
    /// Type-check a .lin source file without producing a binary
    Check(cmd::check::CheckArgs),
    /// Compile and run a .lin source file
    Run(cmd::run::RunArgs),
    /// Run *.test.lin files
    Test(cmd::test::TestArgs),
    /// Watch for file changes and re-run a command
    Watch(cmd::watch::WatchArgs),
    /// Remove .lin-cache directories and build artefacts
    Clean(cmd::clean::CleanArgs),
    /// Format source files (not yet implemented)
    #[command(hide = true)]
    Fmt,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Build(args) => cmd::build::run(&args),
        Commands::Check(args) => cmd::check::run(&args),
        Commands::Run(args) => cmd::run::run(&args),
        Commands::Test(args) => cmd::test::run(&args),
        Commands::Watch(args) => cmd::watch::run(&args),
        Commands::Clean(args) => cmd::clean::run(&args),
        Commands::Fmt => {
            eprintln!("lin fmt: not yet implemented");
            std::process::exit(1);
        }
    }
}
