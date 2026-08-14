fn main() {
    // Propagate TARGET to compile time so option_env!("TARGET") works
    // in library code (not just build scripts).
    println!(
        "cargo:rustc-env=TARGET={}",
        std::env::var("TARGET").unwrap_or_default()
    );
}
