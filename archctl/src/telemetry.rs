use anyhow::Result;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

pub fn init() -> Result<()> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("archctl=info,warn"));

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_target(false).with_ansi(std::env::var_os("NO_COLOR").is_none()))
        .try_init()
        .ok();
    Ok(())
}
