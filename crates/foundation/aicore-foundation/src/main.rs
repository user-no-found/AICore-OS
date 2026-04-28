use aicore_foundation::AicoreLayout;

fn main() {
    std::process::exit(run(std::env::args().skip(1).collect()));
}

fn run(args: Vec<String>) -> i32 {
    match args.as_slice() {
        [] => {
            let layout = AicoreLayout::from_system_home();
            println!("AICore Foundation Runtime");
            println!("status: ok");
            println!("global root: {}", layout.state_root.display());
            println!("bin root: {}", layout.bin_root.display());
            println!(
                "foundation metadata: {}",
                installed_label(layout.runtime_foundation_root.join("install.toml").exists())
            );
            println!("protocol: stdio_jsonl");
            0
        }
        [arg] if arg == "--status" => {
            let layout = AicoreLayout::from_system_home();
            println!("AICore Foundation Runtime");
            println!("status: ok");
            println!("global root: {}", layout.state_root.display());
            println!("bin root: {}", layout.bin_root.display());
            println!(
                "foundation metadata: {}",
                installed_label(layout.runtime_foundation_root.join("install.toml").exists())
            );
            println!("protocol: stdio_jsonl");
            0
        }
        _ => {
            eprintln!("用法：aicore-foundation --status");
            1
        }
    }
}

fn installed_label(value: bool) -> &'static str {
    if value { "installed" } else { "missing" }
}
