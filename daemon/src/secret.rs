pub fn token() -> String {
	let mut bytes = [0u8; 32];
	getrandom::fill(&mut bytes).expect("system randomness unavailable");
	bytes.iter().map(|b| format!("{b:02x}")).collect()
}
