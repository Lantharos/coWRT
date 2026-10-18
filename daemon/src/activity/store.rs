use std::collections::{HashMap, HashSet, VecDeque};
use std::net::IpAddr;

use serde::{Deserialize, Serialize};

use super::parse::{self, Event};

const BUCKET_SECS: u64 = 600;
const BUCKETS: usize = 144;
const SITES_PER_DEVICE: usize = 300;
const BLOCKED_SITES: usize = 1000;

#[derive(Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Counts {
	pub lookups: u32,
	pub blocked: u32,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bucket {
	pub time: u64,
	pub lookups: u32,
	pub blocked: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Site {
	pub domain: String,
	pub lookups: u32,
	pub blocked: u32,
}

#[derive(Default, Serialize, Deserialize)]
pub struct Store {
	day: u64,
	devices: HashMap<String, HashMap<String, Counts>>,
	blocked: HashMap<String, u32>,
	buckets: VecDeque<Bucket>,
}

fn bump(sites: &mut HashMap<String, Counts>, site: String, blocked: bool) {
	if !sites.contains_key(&site) && sites.len() >= SITES_PER_DEVICE {
		sites.retain(|_, counts| counts.lookups > 1);
		if sites.len() >= SITES_PER_DEVICE {
			return;
		}
	}
	let counts = sites.entry(site).or_default();
	counts.lookups += 1;
	counts.blocked += u32::from(blocked);
}

impl Store {
	pub fn roll_over(&mut self, day: u64) {
		if self.day != day {
			self.day = day;
			self.devices.clear();
			self.blocked.clear();
		}
	}

	pub fn record(&mut self, text: &str, owners: &HashMap<IpAddr, String>, now: u64) {
		let mut seen = HashSet::new();
		let mut lookups = 0;
		let mut blocked = 0;
		for event in text.lines().filter_map(parse::line) {
			let (client, domain, was_blocked) = match event {
				Event::Lookup { client, domain } => (client, domain, false),
				Event::Blocked { client, domain } => (client, domain, true),
			};
			let site = parse::site(domain);
			if !seen.insert((client, site.clone(), was_blocked)) {
				continue;
			}
			if was_blocked {
				blocked += 1;
				if self.blocked.len() < BLOCKED_SITES || self.blocked.contains_key(&site) {
					*self.blocked.entry(site.clone()).or_default() += 1;
				}
			} else {
				lookups += 1;
			}
			if let Some(mac) = owners.get(&client) {
				let sites = self.devices.entry(mac.clone()).or_default();
				if was_blocked {
					if let Some(counts) = sites.get_mut(&site) {
						counts.blocked += 1;
					} else {
						bump(sites, site, true);
					}
				} else {
					bump(sites, site, false);
				}
			}
		}
		self.add_to_bucket(now, lookups, blocked);
	}

	fn add_to_bucket(&mut self, now: u64, lookups: u32, blocked: u32) {
		let time = now / BUCKET_SECS * BUCKET_SECS;
		if self.buckets.back().is_none_or(|bucket| bucket.time != time) {
			if self.buckets.len() == BUCKETS {
				self.buckets.pop_front();
			}
			self.buckets.push_back(Bucket {
				time,
				lookups: 0,
				blocked: 0,
			});
		}
		let bucket = self.buckets.back_mut().expect("bucket just pushed");
		bucket.lookups += lookups;
		bucket.blocked += blocked;
	}

	pub fn sites(&self, macs: &[String], limit: usize) -> (Vec<Site>, Counts) {
		let mut merged: HashMap<&str, Counts> = HashMap::new();
		for sites in macs.iter().filter_map(|mac| self.devices.get(mac)) {
			for (domain, counts) in sites {
				let entry = merged.entry(domain).or_default();
				entry.lookups += counts.lookups;
				entry.blocked += counts.blocked;
			}
		}
		let totals = merged.values().fold(Counts::default(), |sum, c| Counts {
			lookups: sum.lookups + c.lookups,
			blocked: sum.blocked + c.blocked,
		});
		let mut sites: Vec<Site> = merged
			.into_iter()
			.map(|(domain, counts)| Site {
				domain: domain.to_owned(),
				lookups: counts.lookups,
				blocked: counts.blocked,
			})
			.collect();
		sites.sort_by(|a, b| b.lookups.cmp(&a.lookups).then_with(|| a.domain.cmp(&b.domain)));
		sites.truncate(limit);
		(sites, totals)
	}

	pub fn top_blocked(&self, limit: usize) -> Vec<Site> {
		let mut sites: Vec<Site> = self
			.blocked
			.iter()
			.map(|(domain, count)| Site {
				domain: domain.clone(),
				lookups: *count,
				blocked: *count,
			})
			.collect();
		sites.sort_by(|a, b| b.blocked.cmp(&a.blocked).then_with(|| a.domain.cmp(&b.domain)));
		sites.truncate(limit);
		sites
	}

	pub fn buckets(&self) -> Vec<Bucket> {
		self.buckets.iter().copied().collect()
	}
}
