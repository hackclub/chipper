use std::path::Path;
use std::process::Command;

fn bun(args: &[&str]) {
    let status = Command::new("bun")
        .args(args)
        .current_dir("frontend")
        .status()
        .unwrap_or_else(|e| panic!("failed to run bun (is it on PATH?): {e}"));
    assert!(status.success(), "bun {} failed", args.join(" "));
}

fn main() {
    for path in [
        "frontend/src",
        "frontend/static",
        "frontend/package.json",
        "frontend/bun.lock",
        "frontend/vite.config.ts",
        "frontend/tsconfig.json",
    ] {
        if Path::new(path).exists() {
            println!("cargo:rerun-if-changed={path}");
        }
    }

    bun(&["install", "--frozen-lockfile"]);
    bun(&["run", "build"]);
}
