use crate::config::U64_BYTES;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

pub fn preview_u64_file(path: &Path, count: usize) -> io::Result<(Vec<u64>, Vec<u64>)> {
	let file = File::open(path)?;
	let file_size = file.metadata()?.len() as usize;
	if file_size % U64_BYTES != 0 {
		return Err(io::Error::new(
			io::ErrorKind::InvalidData,
			"file size must be a multiple of 8 bytes",
		));
	}

	let total_values = file_size / U64_BYTES;
	if total_values == 0 {
		return Ok((Vec::new(), Vec::new()));
	}

	let read_count = count.min(total_values);
	let mut reader = io::BufReader::new(file);
	let mut first = Vec::with_capacity(read_count);
	let mut buffer = [0u8; U64_BYTES];

	for _ in 0..read_count {
		reader.read_exact(&mut buffer)?;
		first.push(u64::from_le_bytes(buffer));
	}

	let mut tail_reader = reader.into_inner();
	tail_reader.seek(SeekFrom::Start(((total_values - read_count) * U64_BYTES) as u64))?;
	let mut tail_reader = io::BufReader::new(tail_reader);
	let mut last = Vec::with_capacity(read_count);

	for _ in 0..read_count {
		tail_reader.read_exact(&mut buffer)?;
		last.push(u64::from_le_bytes(buffer));
	}

	Ok((first, last))
}

pub fn format_u64s(values: &[u64]) -> String {
	values
		.iter()
		.map(|value| value.to_string())
		.collect::<Vec<_>>()
		.join(", ")
}