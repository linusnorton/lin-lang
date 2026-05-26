//! Binary production pipeline for Lin.
//! Orchestrates: source -> lex -> parse -> type check -> LLVM codegen -> link -> binary.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use inkwell::context::Context;
use lin_check::typed_ir::TypedModule;
use lin_check::types::Type;
use lin_check::{Checker, ModuleSignature};
use lin_codegen::Codegen;
use lin_lex::Lexer;
use lin_parse::ast::{Module, Stmt};
use lin_parse::Parser;

#[derive(Debug)]
pub struct CompileOptions {
    pub source_path: PathBuf,
    pub output_path: PathBuf,
    pub emit_ir: bool,
    pub optimize: bool,
}

#[derive(Debug)]
pub enum CompileError {
    Io(std::io::Error),
    TypeCheck(Vec<lin_common::Diagnostic>),
    Codegen(String),
    Link(String),
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileError::Io(e) => write!(f, "I/O error: {}", e),
            CompileError::TypeCheck(diags) => {
                for d in diags {
                    writeln!(f, "type error: {}", d.message)?;
                }
                Ok(())
            }
            CompileError::Codegen(msg) => write!(f, "codegen error: {}", msg),
            CompileError::Link(msg) => write!(f, "link error: {}", msg),
        }
    }
}

impl From<std::io::Error> for CompileError {
    fn from(e: std::io::Error) -> Self {
        CompileError::Io(e)
    }
}

pub fn compile(opts: &CompileOptions) -> Result<(), CompileError> {
    // 1. Read source
    let source = std::fs::read_to_string(&opts.source_path)?;
    let module_name = opts.source_path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let base_dir = opts.source_path.parent().unwrap_or(Path::new(".")).to_path_buf();

    // 2. Lex + Parse
    let ast_module = parse_source(&source).map_err(CompileError::TypeCheck)?;

    // 3a. Pre-resolve imports so we know real export types before checking the main module.
    let mut imported_modules: HashMap<String, TypedModule> = HashMap::new();
    pre_resolve_imports_from_ast(&ast_module, &base_dir, &mut imported_modules)?;

    // 3b. Type check main module with pre-resolved import types.
    let typed_module = check_module_with_imports(&ast_module, &imported_modules)
        .map_err(CompileError::TypeCheck)?;

    // 4. LLVM codegen
    let context = Context::create();
    let mut cg = Codegen::new(&context, &module_name);

    // Register imported modules with codegen so import slots get correct function pointers.
    for (path, imp_module) in &imported_modules {
        cg.register_import(path, imp_module);
    }

    cg.compile_module(&typed_module);

    // 5. Emit LLVM IR if requested (before verify so we can inspect broken IR)
    if opts.emit_ir {
        let ir_path = opts.output_path.with_extension("ll");
        cg.emit_llvm_ir(&ir_path).map_err(CompileError::Codegen)?;
    }

    if opts.optimize {
        cg.run_optimization_passes().map_err(CompileError::Codegen)?;
    }

    cg.verify().map_err(CompileError::Codegen)?;

    // 6. Emit object file
    let obj_path = opts.output_path.with_extension("o");
    cg.emit_object_file(&obj_path).map_err(CompileError::Codegen)?;

    // 7. Link with runtime
    link(&obj_path, &opts.output_path)?;

    // Clean up the .o file.
    let _ = std::fs::remove_file(&obj_path);

    Ok(())
}

// -------------------------------------------------------------------------
// Module cache
// -------------------------------------------------------------------------

/// Compute the SHA-256 hash of source bytes, returned as a hex string.
fn source_hash(source: &str) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(source.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Try to load a cached `TypedModule` for `source` from `.lin-cache/`.
/// Returns `None` if no cache entry exists or it is unreadable.
fn load_cache(source: &str, base_dir: &Path) -> Option<TypedModule> {
    let hash = source_hash(source);
    let cache_path = base_dir.join(".lin-cache").join(format!("{}.typed", hash));
    let bytes = std::fs::read(&cache_path).ok()?;
    bincode::deserialize(&bytes).ok()
}

/// Save a `TypedModule` to `.lin-cache/` keyed by the SHA-256 of `source`.
/// Silently ignores write errors so cache failures never break the build.
fn save_cache(source: &str, module: &TypedModule, base_dir: &Path) {
    let hash = source_hash(source);
    let cache_dir = base_dir.join(".lin-cache");
    if std::fs::create_dir_all(&cache_dir).is_err() {
        return;
    }
    let cache_path = cache_dir.join(format!("{}.typed", hash));
    if let Ok(bytes) = bincode::serialize(module) {
        let _ = std::fs::write(&cache_path, bytes);
    }
}

/// Save the `ModuleSignature` for a module alongside its TypedModule cache.
/// Keyed by the same source hash; stored as `<hash>.sig`.
fn save_signature(source: &str, sig: &ModuleSignature, base_dir: &Path) {
    let hash = source_hash(source);
    let cache_dir = base_dir.join(".lin-cache");
    if std::fs::create_dir_all(&cache_dir).is_err() {
        return;
    }
    let sig_path = cache_dir.join(format!("{}.sig", hash));
    if let Some(bytes) = sig.to_bytes() {
        let _ = std::fs::write(&sig_path, bytes);
    }
}

/// Load a cached `ModuleSignature` for `source`. Returns `None` if not found or unreadable.
fn load_signature(source: &str, base_dir: &Path) -> Option<ModuleSignature> {
    let hash = source_hash(source);
    let sig_path = base_dir.join(".lin-cache").join(format!("{}.sig", hash));
    let bytes = std::fs::read(&sig_path).ok()?;
    ModuleSignature::from_bytes(&bytes)
}

/// Lex and parse a Lin source string into an AST module.
/// Returns Err with parse diagnostics if any parse errors occurred.
fn parse_source(source: &str) -> Result<Module, Vec<lin_common::Diagnostic>> {
    let tokens = Lexer::new(source, 0).tokenize();
    let mut parser = Parser::new(tokens);
    let module = parser.parse_module();
    if !parser.diagnostics.is_empty() {
        return Err(parser.diagnostics);
    }
    Ok(module)
}

/// Build an import_types map from already-typed imported modules, then type-check `ast_module`.
/// Uses `ModuleSignature` for each import — only needs the public name→type map, not the full IR.
fn check_module_with_imports(
    ast_module: &Module,
    imported_modules: &HashMap<String, TypedModule>,
) -> Result<TypedModule, Vec<lin_common::Diagnostic>> {
    let mut import_type_map: HashMap<(String, String), Type> = HashMap::new();
    for (path, imp_module) in imported_modules {
        let sig = ModuleSignature::from_module(imp_module);
        for (name, ty) in sig.exports {
            import_type_map.insert((path.clone(), name), ty);
        }
    }
    let mut checker = Checker::new();
    checker.import_types = import_type_map;
    checker.check_module(ast_module)
}

/// Embedded stdlib source files (mirrors interpreter's include_str! approach).
fn stdlib_source(path: &str) -> Option<&'static str> {
    match path {
        "std/io"     => Some(include_str!("../../../stdlib/io.lin")),
        "std/string" => Some(include_str!("../../../stdlib/string.lin")),
        "std/number" => Some(include_str!("../../../stdlib/number.lin")),
        "std/array"  => Some(include_str!("../../../stdlib/array.lin")),
        "std/iter"   => Some(include_str!("../../../stdlib/iter.lin")),
        "std/result" => Some(include_str!("../../../stdlib/result.lin")),
        _ => None,
    }
}

/// Recursively type-check all imported modules, populating `cache` in dependency order.
fn pre_resolve_imports_from_ast(
    ast_module: &Module,
    base_dir: &Path,
    cache: &mut HashMap<String, TypedModule>,
) -> Result<(), CompileError> {
    for stmt in &ast_module.statements {
        let Stmt::Import { path, .. } = stmt else { continue };
        if cache.contains_key(path.as_str()) {
            continue;
        }

        let (ast_mod, src_text, imported_base) = if let Some(src) = stdlib_source(path.as_str()) {
            let ast = parse_source(src).map_err(CompileError::TypeCheck)?;
            pre_resolve_imports_from_ast(&ast, base_dir, cache)?;
            (ast, src.to_string(), base_dir.to_path_buf())
        } else {
            let file_path = base_dir.join(format!("{}.lin", path));
            let src = std::fs::read_to_string(&file_path)?;
            let ast = parse_source(&src).map_err(CompileError::TypeCheck)?;
            let imported_base = file_path.parent().unwrap_or(base_dir).to_path_buf();
            pre_resolve_imports_from_ast(&ast, &imported_base, cache)?;
            (ast, src, imported_base)
        };

        // Try cache hit first (skip re-checking if source unchanged).
        if let Some(cached) = load_cache(&src_text, &imported_base) {
            // Ensure signature is also cached (for dependent modules to use).
            if load_signature(&src_text, &imported_base).is_none() {
                let sig = ModuleSignature::from_module(&cached);
                save_signature(&src_text, &sig, &imported_base);
            }
            cache.insert(path.clone(), cached);
            continue;
        }

        let typed = check_module_with_imports(&ast_mod, cache)
            .map_err(CompileError::TypeCheck)?;
        let sig = ModuleSignature::from_module(&typed);
        save_cache(&src_text, &typed, &imported_base);
        save_signature(&src_text, &sig, &imported_base);
        cache.insert(path.clone(), typed);
    }
    Ok(())
}

fn link(obj_path: &Path, output_path: &Path) -> Result<(), CompileError> {
    // Find the lin-runtime static library.
    // When running from a cargo build, it should be in the same target directory.
    let runtime_lib = find_runtime_lib();

    let mut cmd = Command::new("cc");
    cmd.arg(obj_path)
        .arg("-o")
        .arg(output_path);

    if let Some(lib) = &runtime_lib {
        cmd.arg(lib);
    } else {
        // Try to find it relative to the cargo output directory.
        // Fall back to assuming it's installed system-wide (future: pkg-config).
        eprintln!("Warning: lin-runtime library not found, linking may fail");
    }

    // Link system libraries needed by lin-runtime (libc via cc).
    let status = cmd.status().map_err(|e| CompileError::Link(e.to_string()))?;

    if !status.success() {
        return Err(CompileError::Link(format!(
            "linker exited with status {}",
            status
        )));
    }

    Ok(())
}

fn find_runtime_lib() -> Option<PathBuf> {
    // Check standard cargo target directories in order.
    let candidates = [
        // Development build (running from workspace)
        "target/debug/liblin_runtime.a",
        "target/release/liblin_runtime.a",
        "../target/debug/liblin_runtime.a",
        "../target/release/liblin_runtime.a",
    ];

    for candidate in &candidates {
        let path = Path::new(candidate);
        if path.exists() {
            return Some(path.to_path_buf());
        }
    }

    // Try CARGO_MANIFEST_DIR-relative paths (works in tests).
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let base = Path::new(&manifest);
        for candidate in &candidates {
            let path = base.join(candidate);
            if path.exists() {
                return Some(path);
            }
            // Go up one level (workspace root)
            if let Some(parent) = base.parent() {
                let path = parent.join(candidate);
                if path.exists() {
                    return Some(path);
                }
            }
        }
    }

    None
}
