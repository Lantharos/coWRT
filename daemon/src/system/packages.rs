use std::io;

use tokio::process::Command;

async fn apk(args: &[&str]) -> io::Result<bool> {
	Ok(Command::new("apk").args(args).output().await?.status.success())
}

pub async fn names() -> Vec<String> {
	let Ok(output) = Command::new("apk").arg("info").output().await else {
		return Vec::new();
	};
	String::from_utf8_lossy(&output.stdout).lines().map(str::to_owned).collect()
}

pub async fn installed(name: &str) -> bool {
	apk(&["info", "-e", name]).await.unwrap_or(false)
}

pub async fn ensure(names: &[&str]) -> io::Result<()> {
	let mut missing = Vec::new();
	for name in names {
		if !installed(name).await {
			missing.push(*name);
		}
	}
	if missing.is_empty() {
		return Ok(());
	}
	apk(&["update"]).await?;
	let mut args = vec!["add"];
	args.extend(&missing);
	let output = Command::new("apk").args(&args).output().await?;
	if output.status.success() {
		return Ok(());
	}
	Err(io::Error::other(format!(
		"Couldn't install {}: {}",
		missing.join(", "),
		String::from_utf8_lossy(&output.stderr).trim()
	)))
}
