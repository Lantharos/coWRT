use std::collections::HashMap;
use std::sync::LazyLock;

static PREFIXES: LazyLock<HashMap<u32, &'static str>> = LazyLock::new(|| {
	include_str!("vendors.txt")
		.lines()
		.filter_map(|line| line.split_once('\t'))
		.flat_map(|(brand, prefixes)| {
			prefixes
				.split(' ')
				.filter_map(move |prefix| Some((u32::from_str_radix(prefix, 16).ok()?, brand)))
		})
		.collect()
});

fn octets(mac: &str) -> Option<[u8; 3]> {
	let mut parts = mac.split(':').map(|part| u8::from_str_radix(part, 16));
	Some([parts.next()?.ok()?, parts.next()?.ok()?, parts.next()?.ok()?])
}

pub fn private_address(mac: &str) -> bool {
	octets(mac).is_some_and(|[first, ..]| first & 0x02 != 0)
}

pub fn maker(mac: &str) -> Option<&'static str> {
	let [a, b, c] = octets(mac)?;
	PREFIXES
		.get(&(u32::from(a) << 16 | u32::from(b) << 8 | u32::from(c)))
		.copied()
}
