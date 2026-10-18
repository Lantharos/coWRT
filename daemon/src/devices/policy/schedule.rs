use serde::{Deserialize, Serialize};

pub const DAYS: [&str; 7] = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];
const NFT_DAYS: [&str; 7] = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];

#[derive(Clone, Serialize, Deserialize)]
pub struct Schedule {
	pub days: Vec<String>,
	pub from: String,
	pub to: String,
}

fn minutes(time: &str) -> Option<u32> {
	let (hours, minutes) = time.split_once(':')?;
	let (hours, minutes): (u32, u32) = (hours.parse().ok()?, minutes.parse().ok()?);
	(hours < 24 && minutes < 60 && time.len() == 5).then_some(hours * 60 + minutes)
}

impl Schedule {
	pub fn validate(&self) -> Result<(), String> {
		if self.days.is_empty() || !self.days.iter().all(|day| DAYS.contains(&day.as_str())) {
			return Err("Pick at least one day".into());
		}
		match (minutes(&self.from), minutes(&self.to)) {
			(Some(from), Some(to)) if from != to => Ok(()),
			(Some(_), Some(_)) => Err("Start and end can't be the same time".into()),
			_ => Err("Times look like 22:00".into()),
		}
	}

	pub fn parse(entry: &str) -> Option<Self> {
		let (days, range) = entry.split_once(' ')?;
		let (from, to) = range.split_once('-')?;
		let schedule = Self {
			days: days.split(',').map(str::to_owned).collect(),
			from: from.to_owned(),
			to: to.to_owned(),
		};
		schedule.validate().ok().map(|()| schedule)
	}

	pub fn to_uci(&self) -> String {
		format!("{} {}-{}", self.days.join(","), self.from, self.to)
	}

	fn day_set(days: impl Iterator<Item = usize>) -> String {
		let names: Vec<String> = days.map(|day| format!("\"{}\"", NFT_DAYS[day])).collect();
		format!("{{ {} }}", names.join(", "))
	}

	pub fn nft_windows(&self) -> Vec<String> {
		let indexes: Vec<usize> = self
			.days
			.iter()
			.filter_map(|day| DAYS.iter().position(|d| d == day))
			.collect();
		let wraps = minutes(&self.to) < minutes(&self.from);
		if !wraps {
			return vec![format!(
				"meta day {} meta hour \"{}\"-\"{}\"",
				Self::day_set(indexes.into_iter()),
				self.from,
				self.to
			)];
		}
		vec![
			format!(
				"meta day {} meta hour \"{}\"-\"23:59:59\"",
				Self::day_set(indexes.iter().copied()),
				self.from
			),
			format!(
				"meta day {} meta hour \"00:00\"-\"{}\"",
				Self::day_set(indexes.iter().map(|day| (day + 1) % 7)),
				self.to
			),
		]
	}
}
