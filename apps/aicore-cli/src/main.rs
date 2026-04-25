fn main() {
    std::process::exit(aicore_cli::run_from_args(
        std::env::args().skip(1).collect(),
    ));
}
