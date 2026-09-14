use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{exit, Command};

fn main() {
    let args: Vec<String> = env::args().collect();
    let input_path = args
        .get(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("samples/_/_.rsx"));

    let src = fs::read_to_string(&input_path).unwrap_or_else(|e| {
        eprintln!("error: cannot read {}: {e}", input_path.display());
        exit(1);
    });

    // --- 검사 단계 ---
    if let Err(errors) = lib::check(&src) {
        for e in &errors {
            eprintln!("{}", e.message);
        }
        eprintln!("\n[rsx] {} error(s), compilation aborted", errors.len());
        exit(1);
    }

    let rs_src = lib::transpile(&src);

    let out_dir = Path::new("target/rsx");
    fs::create_dir_all(out_dir).unwrap();

    let stem = input_path.file_stem().unwrap().to_string_lossy();
    let rs_path = out_dir.join(format!("{stem}.rs"));
    let bin_path = out_dir.join(stem.as_ref());

    fs::write(&rs_path, &rs_src).unwrap();
    println!("[rsx] transpiled -> {}", rs_path.display());

    println!("[rsx] compiling with rustc...");
    let status = Command::new("rustc")
        .arg(&rs_path)
        .arg("-o")
        .arg(&bin_path)
        .status()
        .unwrap_or_else(|e| {
            eprintln!("error: failed to run rustc: {e}");
            exit(1);
        });

    if !status.success() {
        eprintln!("[rsx] compile failed — see {}", rs_path.display());
        exit(status.code().unwrap_or(1));
    }

    println!("[rsx] running {}", bin_path.display());
    let run_status = Command::new(bin_path.canonicalize().unwrap())
        .status()
        .unwrap();
    exit(run_status.code().unwrap_or(0));
}
