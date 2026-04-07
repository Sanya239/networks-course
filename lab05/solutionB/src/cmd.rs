use anyhow::Result;
use std::io;
use std::io::ErrorKind;
use std::str::from_utf8;
use tokio::process::Command;

async fn run_command(cmd: &str) -> Result<String> {
    let output = Command::new("sh").arg("-c").arg(cmd).output().await?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    Ok(format!(
        "--- stdout ---\n{}\n--- stderr ---\n{}",
        stdout, stderr
    ))
}

pub async fn exec(cmd: Vec<u8>) -> Result<String> {
    match run_command(from_utf8(cmd.as_slice())?).await {
        Ok(out) => Ok(out),
        Err(e) => Err(io::Error::new(ErrorKind::Unsupported, format!("error: {e}")).into()),
    }
}
