use std::path::PathBuf;
use std::time::Instant;

#[derive(clap::Args)]
pub struct BuildArgs {
    /// Source file to compile
    pub file: PathBuf,
    /// Output binary path (default: source filename stem)
    #[arg(short, long)]
    pub output: Option<PathBuf>,
    /// Emit LLVM IR alongside the binary
    #[arg(long)]
    pub emit_ir: bool,
    /// Disable optimisation passes
    #[arg(long)]
    pub no_opt: bool,
    /// Show build timing
    #[arg(long)]
    pub verbose: bool,
}

pub fn run(args: &BuildArgs) {
    use lin_compile::{compile, CompileOptions, CompileError};
    use std::fs;
    use std::process;

    let output = args.output.clone().unwrap_or_else(|| {
        PathBuf::from(
            args.file
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .as_ref(),
        )
    });

    let opts = CompileOptions {
        source_path: args.file.clone(),
        output_path: output.clone(),
        emit_ir: args.emit_ir || std::env::var("LIN_EMIT_IR").is_ok(),
        optimize: !(args.no_opt || std::env::var("LIN_NO_OPT").is_ok()),
        coverage: false,
    };

    let t = Instant::now();
    match compile(&opts) {
        Ok(()) => {
            if args.verbose {
                eprintln!("Built: {} ({:.2}s)", output.display(), t.elapsed().as_secs_f64());
            } else {
                eprintln!("Built: {}", output.display());
            }
        }
        Err(CompileError::TypeCheck(diagnostics)) => {
            let source = fs::read_to_string(&args.file).unwrap_or_default();
            let path = args.file.display().to_string();
            for diag in &diagnostics {
                diag.render(&path, &source);
            }
            process::exit(1);
        }
        Err(e) => {
            eprintln!("Build failed: {}", e);
            process::exit(1);
        }
    }
}
