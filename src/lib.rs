pub mod config;
pub mod inspect;
pub mod io;
pub mod phase1;
pub mod phase2;

use crate::inspect::preview_u64_file;
use crate::phase1::build_sorted_runs;
use crate::phase2::merge_sorted_runs;
use std::io::{ErrorKind, Result};
use std::path::Path;

#[derive(Clone, Copy, Debug)]
pub struct SortConfig {
	pub thread_count: usize,
	pub fan_in_factor: usize,
}

#[derive(Debug)]
pub struct SortStats {
	pub phase1_seconds: f64,
	pub phase2_seconds: f64,
	pub total_seconds: f64,
	pub phase1_throughput_mb_s: f64,
	pub phase2_throughput_mb_s: f64,
	pub overall_throughput_mb_s: f64,
	pub runs_written: usize,
	pub merge_passes: usize,
	pub input_bytes: u64,
	pub output_bytes: u64,
}

#[derive(Debug)]
pub struct SortOutputPreview {
	pub first_values: Vec<u64>,
	pub last_values: Vec<u64>,
}

#[derive(Debug)]
pub struct SortReport {
	pub stats: SortStats,
	pub output_preview: SortOutputPreview,
}

pub fn run_external_sort(
	input_path: &Path,
	work_dir: &Path,
	output_path: &Path,
	config: SortConfig,
) -> Result<SortReport> {
	let sort_start = std::time::Instant::now();
	let phase1_stats = build_sorted_runs(input_path, work_dir, config.thread_count)?;
	let phase1_throughput_mb_s = (phase1_stats.input_bytes as f64 / 1_048_576.0) / phase1_stats.elapsed_seconds;
	let merge_stats = merge_sorted_runs(phase1_stats.run_paths.clone(), work_dir, output_path, config.fan_in_factor)?;
	let output_bytes = std::fs::metadata(output_path)?.len();
	let phase2_throughput_mb_s = (output_bytes as f64 / 1_048_576.0) / merge_stats.elapsed_seconds;
	let total_seconds = sort_start.elapsed().as_secs_f64();
	let overall_throughput_mb_s = ((phase1_stats.input_bytes + output_bytes) as f64 / 1_048_576.0) / total_seconds;
	let (first_values, last_values) = preview_u64_file(output_path, crate::config::PREVIEW_COUNT)?;

	Ok(SortReport {
		stats: SortStats {
			phase1_seconds: phase1_stats.elapsed_seconds,
			phase2_seconds: merge_stats.elapsed_seconds,
			total_seconds,
			phase1_throughput_mb_s,
			phase2_throughput_mb_s,
			overall_throughput_mb_s,
			runs_written: phase1_stats.run_paths.len(),
			merge_passes: merge_stats.passes,
			input_bytes: phase1_stats.input_bytes,
			output_bytes,
		},
		output_preview: SortOutputPreview {
			first_values,
			last_values,
		},
	})
}

pub fn remove_path_if_exists(path: &Path) -> Result<()> {
	match std::fs::remove_file(path) {
		Ok(()) => Ok(()),
		Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
		Err(err) => Err(err),
	}
}

pub fn ensure_parent_dir(path: &Path) -> Result<()> {
	if let Some(parent) = path.parent() {
		std::fs::create_dir_all(parent)?;
	}
	Ok(())
}