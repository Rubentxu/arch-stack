use crate::cli::RenderFormat;
use crate::Filesystem;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tracing::{debug, info};

pub fn run(source: PathBuf, format: RenderFormat, out: Option<PathBuf>, kroki_url: &str, fs: &dyn Filesystem) -> Result<i32> {
    if !source.exists() {
        anyhow::bail!("source not found: {}", source.display());
    }
    let fmt = match format {
        RenderFormat::Auto => detect_format(&source),
        RenderFormat::Structurizr => "structurizr",
        RenderFormat::Plantuml => "plantuml",
    };

    let stem = source
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("out");
    let out_dir = out.unwrap_or_else(|| {
        source
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".archctl-rendered")
    });
    fs.create_dir_all(&out_dir).with_context(|| format!("mkdir {}", out_dir.display()))?;
    let out_path = out_dir.join(format!("{stem}.svg"));

    let body = fs.read_to_string(&source).with_context(|| format!("read {}", source.display()))?;
    let url = format!("{kroki_url}/{fmt}/svg");
    debug!(%url, "POST to kroki");
    info!(source = %source.display(), format = fmt, "rendering");

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .context("build reqwest client")?;
    let response = client
        .post(&url)
        .header("Content-Type", "text/plain")
        .body(body)
        .send()
        .with_context(|| format!("POST {url}"))?;
    let status = response.status();
    let bytes = response.bytes().context("read response body")?;
    fs.write(&out_path, &bytes).with_context(|| format!("write {}", out_path.display()))?;

    let ok = status.is_success();
    let payload = serde_json::json!({
        "ok": ok,
        "format": fmt,
        "source": source.display().to_string(),
        "output": out_path.display().to_string(),
        "status": status.as_u16(),
    });
    println!("{payload}");
    Ok(if ok { 0 } else { 1 })
}

fn detect_format(source: &Path) -> &'static str {
    let ext = source.extension().and_then(|e| e.to_str()).unwrap_or("");
    match ext.to_ascii_lowercase().as_str() {
        "puml" | "iuml" | "wsd" => "plantuml",
        _ => "structurizr",
    }
}
