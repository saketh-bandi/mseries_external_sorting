use external_sort::{ensure_parent_dir, remove_path_if_exists, run_external_sort, SortConfig};
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;

const THREAD_COUNTS: &[usize] = &[1, 2, 4, 6, 8, 10];
const FAN_IN_FACTORS: &[usize] = &[2, 4, 8, 16];

fn parse_arg(index: usize, default_value: Option<&str>, args: &[String], name: &str) -> String {
	args.get(index)
		.cloned()
		.or_else(|| default_value.map(|value| value.to_string()))
		.unwrap_or_else(|| {
			eprintln!("usage: cargo run --bin benchmark -- <input.bin> <work_dir> <csv_path>");
			eprintln!("missing {name}");
			std::process::exit(1);
		})
}

fn write_csv_header(file: &mut File) -> io::Result<()> {
	writeln!(
		file,
		"thread_count,fan_in_factor,phase1_seconds,phase2_seconds,total_seconds,phase1_throughput_mb_s,phase2_throughput_mb_s,overall_throughput_mb_s,runs_written,merge_passes,input_bytes,output_bytes"
	)
}

fn write_csv_row(file: &mut File, thread_count: usize, fan_in_factor: usize, report: &external_sort::SortReport) -> io::Result<()> {
	writeln!(
		file,
		"{thread_count},{fan_in_factor},{:.6},{:.6},{:.6},{:.2},{:.2},{:.2},{},{},{},{}",
		report.stats.phase1_seconds,
		report.stats.phase2_seconds,
		report.stats.total_seconds,
		report.stats.phase1_throughput_mb_s,
		report.stats.phase2_throughput_mb_s,
		report.stats.overall_throughput_mb_s,
		report.stats.runs_written,
		report.stats.merge_passes,
		report.stats.input_bytes,
		report.stats.output_bytes
	)
}

fn main() {
	let args: Vec<String> = std::env::args().skip(1).collect();
	let input_path = PathBuf::from(parse_arg(0, None, &args, "input.bin"));
	let work_dir = PathBuf::from(parse_arg(1, None, &args, "work_dir"));
	let csv_path = PathBuf::from(parse_arg(2, None, &args, "csv_path"));

	if let Err(err) = ensure_parent_dir(&csv_path) {
		eprintln!("failed to prepare csv path: {err}");
		std::process::exit(1);
	}
	if let Err(err) = remove_path_if_exists(&csv_path) {
		eprintln!("failed to clear csv path: {err}");
		std::process::exit(1);
	}

	let mut csv_file = OpenOptions::new()
		.create(true)
		.write(true)
		.truncate(true)
		.open(&csv_path)
		.unwrap_or_else(|err| {
			eprintln!("failed to open csv file: {err}");
			std::process::exit(1);
		});
	write_csv_header(&mut csv_file).unwrap_or_else(|err| {
		eprintln!("failed to write csv header: {err}");
		std::process::exit(1);
	});

	if let Err(err) = std::fs::create_dir_all(&work_dir) {
		eprintln!("failed to create work dir: {err}");
		std::process::exit(1);
	}

	for &thread_count in THREAD_COUNTS {
		for &fan_in_factor in FAN_IN_FACTORS {
			let trial_work_dir = work_dir.join(format!("threads_{thread_count}_fan_{fan_in_factor}"));
			if let Err(err) = std::fs::create_dir_all(&trial_work_dir) {
				eprintln!("failed to create trial work dir: {err}");
				std::process::exit(1);
			}
			let output_path = trial_work_dir.join("final_sorted.bin");
			if let Err(err) = remove_path_if_exists(&output_path) {
				eprintln!("failed to clear output file: {err}");
				std::process::exit(1);
			}

			let report = match run_external_sort(
				&input_path,
				&trial_work_dir,
				&output_path,
				SortConfig {
					thread_count,
					fan_in_factor,
				},
			) {
				Ok(report) => report,
				Err(err) => {
					eprintln!("benchmark failed for thread_count={thread_count}, fan_in_factor={fan_in_factor}: {err}");
					std::process::exit(1);
				}
			};

			write_csv_row(&mut csv_file, thread_count, fan_in_factor, &report).unwrap_or_else(|err| {
				eprintln!("failed to write csv row: {err}");
				std::process::exit(1);
			});
			println!("recorded thread_count={thread_count}, fan_in_factor={fan_in_factor}");
			let _ = std::fs::remove_file(&output_path);
			let _ = std::fs::remove_dir_all(&trial_work_dir);
		}
	}
	println!("benchmark csv written to {}", csv_path.display());
}