use std::collections::HashSet;
use std::io;
use std::path::PathBuf;

use crate::outbound::Request;
use crate::state::RUNTIME_DIR;

const LIMIT: u64 = 64 * 1024 * 1024;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum List {
	Ads,
	Threats,
}

impl List {
	pub const ALL: [List; 2] = [List::Ads, List::Threats];

	fn url(self) -> &'static str {
		match self {
			Self::Ads => "https://cdn.jsdelivr.net/gh/hagezi/dns-blocklists@latest/dnsmasq/pro.txt",
			Self::Threats => "https://cdn.jsdelivr.net/gh/hagezi/dns-blocklists@latest/dnsmasq/tif.mini.txt",
		}
	}

	fn file(self) -> PathBuf {
		PathBuf::from(RUNTIME_DIR).join("dns").join(match self {
			Self::Ads => "ads.txt",
			Self::Threats => "threats.txt",
		})
	}
}

pub async fn download(list: List) -> io::Result<usize> {
	let body = Request::get(list.url()).bytes(LIMIT).await?;
	let text = String::from_utf8_lossy(&body);
	let count = text.lines().filter(|line| line.starts_with("local=/")).count();
	if count == 0 {
		return Err(io::Error::other("The list came back empty"));
	}
	let file = list.file();
	tokio::fs::create_dir_all(file.parent().expect("list directory")).await?;
	let temporary = file.with_extension("tmp");
	tokio::fs::write(&temporary, body).await?;
	tokio::fs::rename(temporary, file).await?;
	Ok(count)
}

pub fn downloaded(list: List) -> bool {
	list.file().exists()
}

fn allowed(domain: &str, allow: &HashSet<String>) -> bool {
	let mut rest = domain;
	loop {
		if allow.contains(rest) {
			return true;
		}
		match rest.split_once('.') {
			Some((_, parent)) => rest = parent,
			None => return false,
		}
	}
}

pub async fn compile(lists: &[List], allow: &[String], target: &PathBuf) -> io::Result<usize> {
	let allow: HashSet<String> = allow
		.iter()
		.map(|d| d.trim().trim_matches('.').to_ascii_lowercase())
		.collect();
	let mut out = String::new();
	let mut count = 0;
	for list in lists {
		let Ok(text) = tokio::fs::read_to_string(list.file()).await else {
			continue;
		};
		for line in text.lines() {
			let Some(domain) = line.strip_prefix("local=/").and_then(|rest| rest.strip_suffix('/')) else {
				continue;
			};
			if allowed(domain, &allow) {
				continue;
			}
			out.push_str(line);
			out.push('\n');
			count += 1;
		}
	}
	tokio::fs::write(target, out).await?;
	Ok(count)
}
