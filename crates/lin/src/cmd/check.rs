use std::path::PathBuf;

#[derive(clap::Args)]
pub struct CheckArgs {
    /// Source file to type-check
    pub file: PathBuf,
}

pub fn run(args: &CheckArgs) {
    use std::fs;
    use std::process;

    let path = args.file.display().to_string();
    let source = fs::read_to_string(&args.file).unwrap_or_else(|e| {
        eprintln!("Error reading {}: {}", path, e);
        process::exit(1);
    });

    let mut lexer = lin_lex::Lexer::new(&source, 0);
    let tokens = lexer.tokenize();
    let mut parser = lin_parse::Parser::new(tokens);
    let module = parser.parse_module();

    if !parser.diagnostics.is_empty() {
        for diag in &parser.diagnostics {
            diag.render(&path, &source);
        }
        process::exit(1);
    }

    let mut checker = lin_check::Checker::new();
    match checker.check_module(&module) {
        Ok(_) => eprintln!("Type check passed."),
        Err(diagnostics) => {
            for diag in &diagnostics {
                diag.render(&path, &source);
            }
            process::exit(1);
        }
    }
}
