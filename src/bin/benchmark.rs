use external_sort::{
	dataset::{generate_u64_file, DataDistribution},
	ensure_parent_dir,
	remove_path_if_exists,
	run_external_sort,
	SortConfig,
};
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;

const THREAD_COUNTS: &[usize] = &[1, 2, 4, 6, 8, 10];
const FAN_IN_FACTORS: &[usize] = &[2, 4, 8, 16];

fn parse_csv_usize_list(raw: &str, name: &str) -> Vec<usize> {
	raw.split(',')
		.filter(|value| !value.trim().is_empty())
		.map(|value| {
			value.trim().parse::<usize>().unwrap_or_else(|_| {
				eprintln!("invalid {name} entry: {value}");
				std::process::exit(1);
			})
		})
		.collect()
}

fn parse_csv_distribution_list(raw: &str) -> Vec<DataDistribution> {
	raw.split(',')
		.filter(|value| !value.trim().is_empty())
		.map(|value| {
			DataDistribution::parse(value.trim()).unwrap_or_else(|| {
				eprintln!("invalid distribution entry: {value}");
				std::process::exit(1);
			})
		})
		.collect()
}

fn write_csv_header(file: &mut File) -> io::Result<()> {
	writeln!(
		file,
		"size_mb,distribution,thread_count,fan_in_factor,phase1_seconds,phase2_seconds,total_seconds,phase1_throughput_mb_s,phase2_throughput_mb_s,overall_throughput_mb_s,runs_written,merge_passes,input_bytes,output_bytes"
	)
}

fn write_csv_row(
	file: &mut File,
	size_mb: usize,
	distribution: DataDistribution,
	thread_count: usize,
	fan_in_factor: usize,
	report: &external_sort::SortReport,
) -> io::Result<()> {
	writeln!(
		file,
		"{size_mb},{},{thread_count},{fan_in_factor},{:.6},{:.6},{:.6},{:.2},{:.2},{:.2},{},{},{},{}",
		distribution.as_str(),
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
	let work_dir = PathBuf::from(args.get(0).cloned().unwrap_or_else(|| {
		eprintln!("usage: cargo run --bin benchmark -- <work_dir> <csv_path> [--sizes-mb 16,400] [--distributions random] [--thread-counts 1,2,4,6,8,10] [--fan-in-factors 2,4,8,16]");
		std::process::exit(1);
	}));
	let csv_path = PathBuf::from(args.get(1).cloned().unwrap_or_else(|| {
		eprintln!("usage: cargo run --bin benchmark -- <work_dir> <csv_path> [--sizes-mb 16,400] [--distributions random] [--thread-counts 1,2,4,6,8,10] [--fan-in-factors 2,4,8,16]");
		std::process::exit(1);
	}));

	let mut sizes_mb = vec![16usize, 400usize];
	let mut distributions = vec![
		DataDistribution::Random,
		DataDistribution::Ascending,
		DataDistribution::Descending,
	];
	let mut thread_counts = THREAD_COUNTS.to_vec();
	let mut fan_in_factors = FAN_IN_FACTORS.to_vec();
	let mut index = 2;
	while index < args.len() {
		match args[index].as_str() {
			"--sizes-mb" => {
				let raw = args.get(index + 1).unwrap_or_else(|| {
					eprintln!("missing value for --sizes-mb");
					std::process::exit(1);
				});
				sizes_mb = parse_csv_usize_list(raw, "sizes_mb");
				index += 2;
			}
			"--distributions" => {
				let raw = args.get(index + 1).unwrap_or_else(|| {
					eprintln!("missing value for --distributions");
					std::process::exit(1);
				});
				distributions = parse_csv_distribution_list(raw);
				index += 2;
			}
			"--thread-counts" => {
				let raw = args.get(index + 1).unwrap_or_else(|| {
					eprintln!("missing value for --thread-counts");
					std::process::exit(1);
				});
				thread_counts = parse_csv_usize_list(raw, "thread_counts");
				index += 2;
			}
			"--fan-in-factors" => {
				let raw = args.get(index + 1).unwrap_or_else(|| {
					eprintln!("missing value for --fan-in-factors");
					std::process::exit(1);
				});
				fan_in_factors = parse_csv_usize_list(raw, "fan_in_factors");
				index += 2;
			}
			other => {
				eprintln!("unknown argument: {other}");
				std::process::exit(1);
			}
		}
	}

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

	for &size_mb in &sizes_mb {
		for &distribution in &distributions {
			let dataset_dir = work_dir.join(format!("dataset_{size_mb}mb_{}", distribution.as_str()));
			if let Err(err) = std::fs::create_dir_all(&dataset_dir) {
				eprintln!("failed to create dataset dir: {err}");
				std::process::exit(1);
			}
			let input_path = dataset_dir.join("input.bin");
			if let Err(err) = remove_path_if_exists(&input_path) {
				eprintln!("failed to clear dataset file: {err}");
				std::process::exit(1);
			}
			if let Err(err) = generate_u64_file(
				&input_path,
				size_mb * 1024 * 1024,
				distribution,
				None,
			) {
				eprintln!("failed to generate dataset size={size_mb}MB distribution={}: {err}", distribution.as_str());
				std::process::exit(1);
			}

			for &thread_count in &thread_counts {
				for &fan_in_factor in &fan_in_factors {
					let trial_work_dir = dataset_dir.join(format!("threads_{thread_count}_fan_{fan_in_factor}"));
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
							eprintln!("benchmark failed for size={size_mb}MB distribution={} thread_count={thread_count} fan_in_factor={fan_in_factor}: {err}", distribution.as_str());
							std::process::exit(1);
						}
					};

					write_csv_row(&mut csv_file, size_mb, distribution, thread_count, fan_in_factor, &report).unwrap_or_else(|err| {
						eprintln!("failed to write csv row: {err}");
						std::process::exit(1);
					});
					println!("recorded size={size_mb}MB distribution={} thread_count={thread_count} fan_in_factor={fan_in_factor}", distribution.as_str());
					let _ = std::fs::remove_file(&output_path);
					let _ = std::fs::remove_dir_all(&trial_work_dir);
				}
			}
		}
	}
	println!("benchmark csv written to {}", csv_path.display());
}