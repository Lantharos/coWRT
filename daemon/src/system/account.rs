use std::process::Stdio;

use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use crate::http::ApiError;

const MIN_LENGTH: usize = 8;

pub async fn has_password() -> bool {
	let shadow = tokio::fs::read_to_string("/etc/shadow").await.unwrap_or_default();
	shadow
		.lines()
		.find_map(|line| line.strip_prefix("root:"))
		.and_then(|rest| rest.split(':').next())
		.is_some_and(|hash| !hash.is_empty() && hash != "!" && hash != "*")
}

pub async fn set_password(next: &str) -> Result<(), ApiError> {
	if next.len() < MIN_LENGTH {
		return Err(ApiError::bad_request("Use at least 8 characters"));
	}
	if next.contains('\n') {
		return Err(ApiError::bad_request("Passwords can't contain line breaks"));
	}
	let mut child = Command::new("passwd")
		.arg("root")
		.stdin(Stdio::piped())
		.stdout(Stdio::null())
		.stderr(Stdio::piped())
		.spawn()?;
	child
		.stdin
		.take()
		.expect("piped stdin")
		.write_all(format!("{next}\n{next}\n").as_bytes())
		.await?;
	let output = child.wait_with_output().await?;
	if !output.status.success() {
		return Err(ApiError::conflict(String::from_utf8_lossy(&output.stderr).trim().to_owned()));
	}
	Ok(())
}
