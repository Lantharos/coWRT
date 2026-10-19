use std::process::Stdio;

use tokio::io::AsyncWriteExt;
use tokio::process::Command;

pub async fn run(ruleset: &str) -> std::io::Result<()> {
	let timezone = tokio::fs::read_to_string("/etc/TZ").await.unwrap_or_default();
	let mut child = Command::new("nft")
		.args(["-f", "-"])
		.env("TZ", timezone.trim())
		.stdin(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn()?;
	child.stdin.take().expect("piped stdin").write_all(ruleset.as_bytes()).await?;
	let output = child.wait_with_output().await?;
	if output.status.success() {
		return Ok(());
	}
	Err(std::io::Error::other(
		String::from_utf8_lossy(&output.stderr).trim().to_owned(),
	))
}
