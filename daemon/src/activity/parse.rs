use std::net::IpAddr;

pub enum Event<'a> {
	Lookup { client: IpAddr, domain: &'a str },
	Blocked { client: IpAddr, domain: &'a str },
}

pub fn line(line: &str) -> Option<Event<'_>> {
	let (_, rest) = line.split_once("]: ")?;
	let mut words = rest.split_ascii_whitespace();
	let _serial = words.next()?;
	let client = words.next()?.rsplit_once('/')?.0.parse().ok()?;
	let verb = words.next()?;
	let domain = words.next()?;
	if verb.starts_with("query[") {
		return Some(Event::Lookup { client, domain });
	}
	if verb == "config" && words.next() == Some("is") {
		return match words.next()? {
			"NXDOMAIN" | "0.0.0.0" | "::" => Some(Event::Blocked { client, domain }),
			_ => None,
		};
	}
	None
}

const SECOND_LEVEL: [&str; 12] = ["co", "com", "net", "org", "ac", "gov", "edu", "ne", "or", "go", "gv", "nic"];

pub fn site(domain: &str) -> String {
	let domain = domain.trim_end_matches('.').to_ascii_lowercase();
	let labels: Vec<&str> = domain.split('.').collect();
	let keep = match labels.as_slice() {
		[.., second, top] if top.len() == 2 && SECOND_LEVEL.contains(second) => 3,
		_ => 2,
	};
	labels[labels.len().saturating_sub(keep)..].join(".")
}
