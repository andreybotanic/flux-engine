fn main() {
    if let Err(error) = xtask::run_from_env() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
