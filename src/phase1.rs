use crate::config::{max_chunk_values, READ_BUFFER_BYTES, U64_BYTES};
use crate::io::{open_run_writer, write_u64_run};
use rayon::prelude::*;
use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::{Path, PathBuf};

pub struct Phase1Stats {
	pub run_paths: Vec<PathBuf>,
	pub elapsed_seconds: f64,
	pub input_bytes: u64,
}

fn spill_run(run_dir: &Path, run_index: usize, chunk: &mut Vec<u64>) -> io::Result<PathBuf> {
	chunk.par_sort_unstable();

	let run_path = run_dir.join(format!("run_{run_index:05}.bin"));
	let mut writer = open_run_writer(&run_path)?;
	write_u64_run(&mut writer, chunk)?;
	chunk.clear();

	Ok(run_path)
}

pub fn build_sorted_runs(input_path: &Path, run_dir: &Path) -> io::Result<Phase1Stats> {
	std::fs::create_dir_all(run_dir)?;

	let input_file = File::open(input_path)?;
	let input_size = input_file.metadata()?.len();
	let mut reader = BufReader::with_capacity(READ_BUFFER_BYTES, input_file);
	let mut buffer = [0u8; READ_BUFFER_BYTES];
	let mut chunk: Vec<u64> = Vec::with_capacity(max_chunk_values());
	let mut run_paths = Vec::new();
	let mut run_index = 0usize;
	let start = std::time::Instant::now();

	loop {
		let bytes_read = reader.read(&mut buffer)?;
		if bytes_read == 0 {
			break;
		}

		if bytes_read % U64_BYTES != 0 {
			return Err(io::Error::new(
				io::ErrorKind::InvalidData,
				"input file ended on a partial u64 record",
			));
		}

		for value_bytes in buffer[..bytes_read].chunks_exact(U64_BYTES) {
			let value = u64::from_le_bytes(value_bytes.try_into().unwrap());
			chunk.push(value);

			if chunk.len() == max_chunk_values() {
				run_paths.push(spill_run(run_dir, run_index, &mut chunk)?);
				run_index += 1;
			}
		}
	}

	if !chunk.is_empty() {
		run_paths.push(spill_run(run_dir, run_index, &mut chunk)?);
	}

	Ok(Phase1Stats {
		run_paths,
		elapsed_seconds: start.elapsed().as_secs_f64(),
		input_bytes: input_size,
	})
}