use std::env;
use std::path::PathBuf;

use external_sort::{
	config::{DEFAULT_FAN_IN_FACTOR, DEFAULT_THREAD_COUNT, MEMORY_LIMIT_BYTES, PREVIEW_COUNT},
	ensure_parent_dir,
	inspect::{format_u64s, preview_u64_file},
	run_external_sort,
	remove_path_if_exists,
	SortConfig,
};

fn parse_usize_arg(value: Option<String>, default_value: usize, name: &str) -> usize {
	match value {
		Some(raw) => raw.parse::<usize>().unwrap_or_else(|_| {
			eprintln!("invalid {name}: {raw}");
			std::process::exit(1);
		}),
		None => default_value,
	}
}

fn main() {
	let mut args = env::args().skip(1);
	let input_path = args.next().unwrap_or_else(|| {
		eprintln!("usage: cargo run -- <input.bin> <run_dir> <output.bin> [thread_count] [fan_in_factor]");
		std::process::exit(1);
	});
	let run_dir = args.next().unwrap_or_else(|| {
		eprintln!("usage: cargo run -- <input.bin> <run_dir> <output.bin> [thread_count] [fan_in_factor]");
		std::process::exit(1);
	});
	let output_path = args.next().unwrap_or_else(|| {
		eprintln!("usage: cargo run -- <input.bin> <run_dir> <output.bin> [thread_count] [fan_in_factor]");
		std::process::exit(1);
	});
	let thread_count = parse_usize_arg(args.next(), DEFAULT_THREAD_COUNT, "thread_count");
	let fan_in_factor = parse_usize_arg(args.next(), DEFAULT_FAN_IN_FACTOR, "fan_in_factor");

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

	if let Err(err) = ensure_parent_dir(&output_path) {
		eprintln!("output path setup failed: {err}");
		std::process::exit(1);
	}
	if let Err(err) = remove_path_if_exists(&output_path) {
		eprintln!("failed to clear output path: {err}");
		std::process::exit(1);
	}

	let report = match run_external_sort(
		&input_path,
		&run_dir,
		&output_path,
		SortConfig {
			thread_count,
			fan_in_factor,
		},
	) {
		Ok(report) => report,
		Err(err) => {
			eprintln!("sort failed: {err}");
			std::process::exit(1);
		}
	};

	println!("memory limit bytes: {MEMORY_LIMIT_BYTES}");
	println!("thread count: {}", thread_count);
	println!("fan in factor: {}", fan_in_factor);
	println!("runs written: {}", report.stats.runs_written);
	println!("merge passes: {}", report.stats.merge_passes);
	println!("phase 1 seconds: {:.3}", report.stats.phase1_seconds);
	println!("phase 1 throughput MB/s: {:.2}", report.stats.phase1_throughput_mb_s);
	println!("phase 2 seconds: {:.3}", report.stats.phase2_seconds);
	println!("phase 2 throughput MB/s: {:.2}", report.stats.phase2_throughput_mb_s);
	println!("total sort seconds: {:.3}", report.stats.total_seconds);
	println!("overall throughput MB/s: {:.2}", report.stats.overall_throughput_mb_s);
	println!("output file: {}", output_path.display());
	println!("output first {}: {}", report.output_preview.first_values.len(), format_u64s(&report.output_preview.first_values));
	println!("output last {}: {}", report.output_preview.last_values.len(), format_u64s(&report.output_preview.last_values));
}