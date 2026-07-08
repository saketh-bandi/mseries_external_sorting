use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

use crate::config::U64_BYTES;

pub const PAGE_SIZE_BYTES: usize = 16 * 1024;
pub const WRITE_BLOCK_BYTES: usize = PAGE_SIZE_BYTES * 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataDistribution {
	Random,
	Ascending,
	Descending,
}

impl DataDistribution {
	pub fn parse(value: &str) -> Option<Self> {
		match value {
			"random" => Some(Self::Random),
			"ascending" => Some(Self::Ascending),
			"descending" => Some(Self::Descending),
			_ => None,
		}
	}

	pub fn as_str(&self) -> &'static str {
		match self {
			Self::Random => "random",
			Self::Ascending => "ascending",
			Self::Descending => "descending",
		}
	}
}

#[derive(Clone, Debug)]
pub struct XorShift64Star {
	state: u64,
}

impl XorShift64Star {
	pub fn new(seed: u64) -> Self {
		let state = if seed == 0 { 0x9E3779B97F4A7C15 } else { seed };
		Self { state }
	}

	pub fn next_u64(&mut self) -> u64 {
		let mut x = self.state;
		x ^= x >> 12;
		x ^= x << 25;
		x ^= x >> 27;
		self.state = x;
		x.wrapping_mul(0x2545F4914F6CDD1D)
	}
}

pub fn generate_u64_file(
	output_path: &Path,
	size_bytes: usize,
	distribution: DataDistribution,
	seed: Option<u64>,
) -> io::Result<()> {
	if size_bytes % U64_BYTES != 0 {
		return Err(io::Error::new(
			io::ErrorKind::InvalidInput,
			"size_bytes must be a multiple of 8",
		));
	}
	if WRITE_BLOCK_BYTES % PAGE_SIZE_BYTES != 0 {
		return Err(io::Error::new(
			io::ErrorKind::InvalidInput,
			"write block size must be a multiple of the page size",
		));
	}

	let value_count = size_bytes / U64_BYTES;
	let values_per_block = WRITE_BLOCK_BYTES / U64_BYTES;
	let mut rng = XorShift64Star::new(seed.unwrap_or(0xD1B54A32D192ED03));
	let mut current_value = 1u64;

	output_path.parent().map(std::fs::create_dir_all).transpose()?;
	let mut file = File::create(output_path)?;

	let mut remaining = value_count;
	while remaining > 0 {
		let block_value_count = remaining.min(values_per_block);
		let mut buffer = Vec::with_capacity(block_value_count * U64_BYTES);

		for _ in 0..block_value_count {
			let value = match distribution {
				DataDistribution::Random => rng.next_u64(),
				DataDistribution::Ascending => {
					let value = current_value;
					current_value += 1;
					value
				}
				DataDistribution::Descending => {
					let value = value_count as u64 - current_value + 1;
					current_value += 1;
					value
				}
			};
			buffer.extend_from_slice(&value.to_le_bytes());
		}

		file.write_all(&buffer)?;
		remaining -= block_value_count;
	}

	file.flush()?;
	Ok(())
}
