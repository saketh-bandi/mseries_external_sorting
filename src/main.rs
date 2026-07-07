mod config;
mod inspect;
mod io;
mod phase1;
mod phase2;

use config::{MEMORY_LIMIT_BYTES, PREVIEW_COUNT, THREAD_COUNT};
use inspect::{format_u64s, preview_u64_file};
use phase1::build_sorted_runs;
use phase2::merge_sorted_runs;
use std::env;
use std::path::PathBuf;

fn main() {
	rayon::ThreadPoolBuilder::new()
		.num_threads(THREAD_COUNT)
		.build_global()
		.expect("failed to initialize Rayon thread pool");

	let mut args = env::args().skip(1);
	let input_path = args.next().unwrap_or_else(|| {
		eprintln!("usage: cargo run -- <input.bin> <run_dir> <output.bin>");
		std::process::exit(1);
	});
	let run_dir = args.next().unwrap_or_else(|| {
		eprintln!("usage: cargo run -- <input.bin> <run_dir> <output.bin>");
		std::process::exit(1);
	});
	let output_path = args.next().unwrap_or_else(|| {
		eprintln!("usage: cargo run -- <input.bin> <run_dir> <output.bin>");
		std::process::exit(1);
	});

	let input_path = PathBuf::from(input_path);
	let run_dir = PathBuf::from(run_dir);
	let output_path = PathBuf::from(output_path);

	match preview_u64_file(&input_path, PREVIEW_COUNT) {
		Ok((first_values, last_values)) => {
			println!("input file: {}", input_path.display());
			println!("input first {}: {}", first_values.len(), format_u64s(&first_values));
			println!("input last {}: {}", last_values.len(), format_u64s(&last_values));
		}
		Err(err) => {
			eprintln!("input preview failed: {err}");
			std::process::exit(1);
		}
	}

	let sort_start = std::time::Instant::now();

	match build_sorted_runs(&input_path, &run_dir) {
		Ok(stats) => {
			let phase1_throughput_mb_s = (stats.input_bytes as f64 / 1_048_576.0) / stats.elapsed_seconds;
			println!("phase 1 complete");
			println!("memory limit bytes: {MEMORY_LIMIT_BYTES}");
			println!("runs written: {}", stats.run_paths.len());
			println!("phase 1 seconds: {:.3}", stats.elapsed_seconds);
			println!("phase 1 throughput MB/s: {:.2}", phase1_throughput_mb_s);

			match merge_sorted_runs(stats.run_paths, &run_dir, &output_path) {
				Ok(merge_stats) => {
					let output_bytes = std::fs::metadata(&output_path)
						.map(|metadata| metadata.len())
						.unwrap_or(merge_stats.output_bytes);
					let phase2_throughput_mb_s =
						(output_bytes as f64 / 1_048_576.0) / merge_stats.elapsed_seconds;
					let total_sort_seconds = sort_start.elapsed().as_secs_f64();
					let total_bytes_processed = stats.input_bytes + output_bytes;
					let overall_throughput_mb_s =
						(total_bytes_processed as f64 / 1_048_576.0) / total_sort_seconds;
					println!("phase 2 complete");
					println!("fan in factor: {}", config::FAN_IN_FACTOR);
					println!("merge passes: {}", merge_stats.passes);
					println!("phase 2 seconds: {:.3}", merge_stats.elapsed_seconds);
					println!("phase 2 throughput MB/s: {:.2}", phase2_throughput_mb_s);
					println!("total sort seconds: {:.3}", total_sort_seconds);
					println!("overall throughput MB/s: {:.2}", overall_throughput_mb_s);
					match preview_u64_file(&output_path, PREVIEW_COUNT) {
						Ok((first_values, last_values)) => {
							println!("output file: {}", output_path.display());
							println!("output first {}: {}", first_values.len(), format_u64s(&first_values));
							println!("output last {}: {}", last_values.len(), format_u64s(&last_values));
						}
						Err(err) => {
							eprintln!("preview failed: {err}");
							std::process::exit(1);
						}
					}
				}
				Err(err) => {
					eprintln!("phase 2 failed: {err}");
					std::process::exit(1);
				}
			}
		}
		Err(err) => {
			eprintln!("phase 1 failed: {err}");
			std::process::exit(1);
		}
	}
}