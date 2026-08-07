use anyhow::Result;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use crate::environment::Environment;

pub fn init() -> Result<()> {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("archctl=info,warn"));

    // The Environment port is the boundary; we read NO_COLOR via the
    // SystemEnvironment adapter so tests can inject a fixed answer
    // (see `FixedEnvironment::with_var`).
    let no_color = crate::environment::SystemEnvironment
        .var("NO_COLOR")
        .is_some();

    tracing_subscriber::registry()
        .with(filter)
        .with(
            fmt::layer()
                .with_target(false)
                .with_ansi(no_color)
                // Unix convention: stdout = data, stderr = logs. Without
                // this, `archctl ... --json` consumers see INFO lines
                // before the JSON payload (the pre-existing MEDIUM from
                // M31 debt-verify).
                .with_writer(std::io::stderr),
        )
        .try_init()
        .ok();
    Ok(())
}
