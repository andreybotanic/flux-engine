use std::{fmt, path::PathBuf};

mod wgsl_imports;
mod wgsl_validation;

/// Error returned by the standalone WGSL validation xtask.
#[derive(Debug)]
pub(crate) struct WgslToolError(String);

impl WgslToolError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for WgslToolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for WgslToolError {}

impl From<std::io::Error> for WgslToolError {
    fn from(error: std::io::Error) -> Self {
        Self(error.to_string())
    }
}

fn main() {
    if let Err(error) = run_from_env() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run_from_env() -> Result<(), WgslToolError> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if !args.is_empty() {
        return Err(WgslToolError::new(
            "usage: cargo validate-wgsl (without extra arguments)",
        ));
    }
    let repo_root = std::env::current_dir()?;
    wgsl_validation::run_validate_wgsl_command(&repo_root)
}

pub(crate) fn cargo_binary() -> PathBuf {
    if let Some(path) = std::env::var_os("CARGO") {
        return PathBuf::from(path);
    }
    let fallback = PathBuf::from(r"C:\Users\andreybotanic\.cargo\bin\cargo.exe");
    if fallback.exists() {
        return fallback;
    }
    PathBuf::from("cargo")
}
