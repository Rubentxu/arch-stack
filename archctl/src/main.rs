use clap::Parser;

pub fn main() -> anyhow::Result<()> {
    archctl::telemetry::init()?;
    let cli = archctl::Cli::parse();
    tracing::info!(version = env!("CARGO_PKG_VERSION"), "archctl starting");
    let code = archctl::run(cli)?;
    std::process::exit(code);
}
