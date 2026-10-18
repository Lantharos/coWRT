use std::collections::HashSet;

use serde::Serialize;
use serde_json::{Value, json};

use super::{band_code, ifnames, radio_iface};
use crate::devices::sources;
use crate::http::ApiError;
use crate::platform::config::uci;
use crate::platform::ubus::Ubus;

const DFS_PENALTY: f64 = 1.5;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Neighbor {
	ssid: String,
	channel: u32,
	signal: i64,
	width: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Channel {
	channel: u32,
	networks: usize,
	load: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Scan {
	neighbors: Vec<Neighbor>,
	channels: Vec<Channel>,
	best: Option<u32>,
}

fn frequency(band: &str, channel: u32) -> f64 {
	f64::from(channel) * 5.0
		+ match band {
			"2g" => 2407.0,
			"6g" => 5950.0,
			_ => 5000.0,
		}
}

fn our_width(htmode: Option<&str>, band: &str) -> u32 {
	let width = htmode
		.and_then(|mode| mode.trim_start_matches(|c: char| c.is_ascii_alphabetic()).parse().ok())
		.unwrap_or(20);
	if band == "2g" { 20 } else { width }
}

fn block_center(band: &str, channel: u32, width: u32) -> u32 {
	if band == "2g" || width <= 20 {
		return channel;
	}
	let span = width / 20;
	let base = if band == "5g" && channel >= 149 {
		149
	} else if band == "6g" {
		1
	} else {
		36
	};
	let start = base + (channel.saturating_sub(base) / 4 / span) * span * 4;
	start + (span - 1) * 2
}

fn is_dfs(band: &str, channel: u32) -> bool {
	band == "5g" && (52..=144).contains(&channel)
}

fn neighbor(entry: &Value) -> Option<(Neighbor, u32)> {
	let channel = entry.get("channel")?.as_u64()? as u32;
	let vht = entry.get("vht_operation");
	let width = vht
		.and_then(|v| v.get("channel_width")?.as_u64())
		.or_else(|| entry.pointer("/ht_operation/channel_width")?.as_u64())
		.unwrap_or(20) as u32;
	let center = match vht.and_then(|v| v.get("center_freq_1")?.as_u64()) {
		Some(center) if width >= 80 && center > 0 => center as u32,
		_ => match entry
			.pointer("/ht_operation/secondary_channel_offset")
			.and_then(Value::as_str)
		{
			Some("above") if width >= 40 => channel + 2,
			Some("below") if width >= 40 => channel.saturating_sub(2),
			_ => channel,
		},
	};
	Some((
		Neighbor {
			ssid: entry.get("ssid").and_then(Value::as_str).unwrap_or_default().to_owned(),
			channel,
			signal: entry.get("signal")?.as_i64()?,
			width,
		},
		center,
	))
}

fn overlap(a: (f64, f64), b: (f64, f64)) -> f64 {
	(a.1.min(b.1) - a.0.max(b.0)).max(0.0)
}

pub async fn scan(ubus: &Ubus, radio: &str, candidates: &[u32]) -> Result<Scan, ApiError> {
	let sections = uci::sections(ubus, "wireless").await?;
	let device = sections
		.iter()
		.find(|s| s.kind == "wifi-device" && s.name == radio)
		.ok_or_else(|| ApiError::not_found("That radio doesn't exist"))?;
	let band = device.get("band").unwrap_or("5g");
	let status = ubus.call("network.wireless", "status", json!({})).await.unwrap_or_default();
	let ifnames = ifnames(&status);
	let iface = radio_iface(&sections, &ifnames, radio).ok_or_else(|| ApiError::conflict("Turn the radio on to look around"))?;
	let results = ubus.call("iwinfo", "scan", json!({ "device": iface })).await?;
	let own: HashSet<String> = sources::local_macs().await.into_iter().collect();
	let code = band_code(band);
	let found: Vec<(Neighbor, u32)> = results
		.get("results")
		.and_then(Value::as_array)
		.into_iter()
		.flatten()
		.filter(|entry| entry.get("band").and_then(Value::as_u64) == Some(code))
		.filter(|entry| {
			entry
				.get("bssid")
				.and_then(Value::as_str)
				.is_none_or(|bssid| !own.contains(&bssid.to_ascii_lowercase()))
		})
		.filter_map(neighbor)
		.collect();

	let width = our_width(device.get("htmode"), band);
	let candidates: Vec<u32> = if band == "2g" {
		[1, 6, 11].into_iter().filter(|c| candidates.contains(c)).collect()
	} else {
		candidates.to_vec()
	};
	let mut channels: Vec<Channel> = candidates
		.iter()
		.map(|&channel| {
			let center = frequency(band, block_center(band, channel, width));
			let ours = (center - f64::from(width) / 2.0, center + f64::from(width) / 2.0);
			let mut networks = 0;
			let load = found
				.iter()
				.map(|(n, their_center)| {
					let middle = frequency(band, *their_center);
					let theirs = (middle - f64::from(n.width) / 2.0, middle + f64::from(n.width) / 2.0);
					let shared = overlap(ours, theirs) / f64::from(n.width);
					if shared > 0.0 {
						networks += 1;
					}
					10f64.powf(n.signal as f64 / 10.0) * shared
				})
				.sum();
			Channel { channel, networks, load }
		})
		.collect();
	let best = channels
		.iter()
		.min_by(|a, b| {
			let score = |c: &Channel| c.load * if is_dfs(band, c.channel) { DFS_PENALTY } else { 1.0 };
			score(a)
				.total_cmp(&score(b))
				.then(is_dfs(band, a.channel).cmp(&is_dfs(band, b.channel)))
		})
		.map(|c| c.channel);
	let peak = channels.iter().map(|c| c.load).fold(0.0, f64::max);
	for channel in &mut channels {
		channel.load = if peak > 0.0 { channel.load / peak } else { 0.0 };
	}
	let mut neighbors: Vec<Neighbor> = found.into_iter().map(|(n, _)| n).collect();
	neighbors.sort_by_key(|n| -n.signal);
	Ok(Scan {
		neighbors,
		channels,
		best,
	})
}
