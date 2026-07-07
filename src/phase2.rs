use crate::config::{FAN_IN_FACTOR, READ_BUFFER_BYTES, U64_BYTES};
use crate::io::open_run_writer;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::fs::File;
use std::io::{self, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

pub struct MergeStats {
	pub elapsed_seconds: f64,
	pub passes: usize,
	pub output_bytes: u64,
}

struct RunCursor {
	reader: BufReader<File>,
	current_value: Option<u64>,
}

impl RunCursor {
	fn open(path: &Path) -> io::Result<Self> {
		let file = File::open(path)?;
		let mut cursor = Self {
			reader: BufReader::with_capacity(READ_BUFFER_BYTES, file),
			current_value: None,
		};
		cursor.current_value = cursor.read_next_value()?;
		Ok(cursor)
	}

	fn read_next_value(&mut self) -> io::Result<Option<u64>> {
		let mut buffer = [0u8; U64_BYTES];
		match self.reader.read(&mut buffer)? {
			0 => Ok(None),
			U64_BYTES => Ok(Some(u64::from_le_bytes(buffer))),
			n => Err(io::Error::new(
				io::ErrorKind::InvalidData,
				format!("run file ended on a partial u64 record after {n} bytes"),
			)),
		}
	}

	fn advance(&mut self) -> io::Result<Option<u64>> {
		self.current_value = self.read_next_value()?;
		Ok(self.current_value)
	}
}

fn merge_batch(run_paths: &[PathBuf], output_path: &Path) -> io::Result<u64> {
	if run_paths.is_empty() {
		return Err(io::Error::new(
			io::ErrorKind::InvalidInput,
			"cannot merge an empty set of run files",
		));
	}

	let mut cursors: Vec<RunCursor> = run_paths
		.iter()
		.map(|path| RunCursor::open(path))
		.collect::<io::Result<Vec<_>>>()?;
	let mut heap: BinaryHeap<Reverse<(u64, usize)>> = BinaryHeap::new();

	for (index, cursor) in cursors.iter().enumerate() {
		if let Some(value) = cursor.current_value {
			heap.push(Reverse((value, index)));
		}
	}

	let mut writer = open_run_writer(output_path)?;
	let mut output_bytes = 0u64;

	while let Some(Reverse((value, cursor_index))) = heap.pop() {
		writer.write_all(&value.to_le_bytes())?;
		output_bytes += U64_BYTES as u64;

		if let Some(next_value) = cursors[cursor_index].advance()? {
			heap.push(Reverse((next_value, cursor_index)));
		}
	}

	writer.flush()?;
	Ok(output_bytes)
}

fn merge_pass(run_paths: &[PathBuf], output_dir: &Path, output_path: Option<&Path>, pass_index: usize) -> io::Result<Vec<PathBuf>> {
	let mut next_runs = Vec::new();
	for (batch_index, batch) in run_paths.chunks(FAN_IN_FACTOR).enumerate() {
		let target_path = match output_path {
			Some(final_output) if run_paths.len() <= FAN_IN_FACTOR => final_output.to_path_buf(),
			_ => output_dir.join(format!("merge_{pass_index:05}_{batch_index:05}.bin")),
		};

		merge_batch(batch, &target_path)?;
		if output_path.map(|final_output| final_output == target_path.as_path()).unwrap_or(false) {
			continue;
		}
		next_runs.push(target_path);
	}

	for path in run_paths {
		let _ = std::fs::remove_file(path);
	}

	Ok(next_runs)
}

pub fn merge_sorted_runs(mut run_paths: Vec<PathBuf>, work_dir: &Path, output_path: &Path) -> io::Result<MergeStats> {
	if run_paths.is_empty() {
		return Err(io::Error::new(
			io::ErrorKind::InvalidInput,
			"phase 2 requires at least one run file",
		));
	}

	std::fs::create_dir_all(work_dir)?;
	if let Some(parent) = output_path.parent() {
		std::fs::create_dir_all(parent)?;
	}
	if output_path.exists() {
		std::fs::remove_file(output_path)?;
	}

	let start = Instant::now();
	let mut pass_index = 0usize;

	if run_paths.len() == 1 {
		merge_batch(&run_paths, output_path)?;
		std::fs::remove_file(&run_paths[0])?;
		let output_bytes = std::fs::metadata(output_path)?.len();
		return Ok(MergeStats {
			elapsed_seconds: start.elapsed().as_secs_f64(),
			passes: 1,
			output_bytes,
		});
	}

	while run_paths.len() > 1 {
		if run_paths.len() <= FAN_IN_FACTOR {
			merge_batch(&run_paths, output_path)?;
			for path in &run_paths {
				let _ = std::fs::remove_file(path);
			}
			let output_bytes = std::fs::metadata(output_path)?.len();
			return Ok(MergeStats {
				elapsed_seconds: start.elapsed().as_secs_f64(),
				passes: pass_index + 1,
				output_bytes,
			});
		}

		run_paths = merge_pass(&run_paths, work_dir, None, pass_index)?;
		pass_index += 1;
	}

	unreachable!("merge loop must terminate with a final output file");
}