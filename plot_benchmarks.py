from __future__ import annotations

import argparse
from pathlib import Path
from typing import Dict

import matplotlib.pyplot as plt
import pandas as pd
import seaborn as sns


DEFAULT_INPUT = Path("benchmark_results.csv")
DEFAULT_OUTPUT_DIR = Path("benchmark_plots")
DEFAULT_METRIC = "overall_throughput_mb_s"


def parse_weights(raw: str | None) -> Dict[str, float]:
	if not raw:
		return {}

	weights: Dict[str, float] = {}
	for item in raw.split(","):
		item = item.strip()
		if not item:
			continue
		if "=" not in item:
			raise ValueError(f"invalid weight entry '{item}', expected name=value")
		name, value = item.split("=", 1)
		weights[name.strip()] = float(value.strip())
	return weights


def build_parser() -> argparse.ArgumentParser:
	parser = argparse.ArgumentParser(description="Plot and rank external sorting benchmarks.")
	parser.add_argument("--input", type=Path, default=DEFAULT_INPUT, help="Benchmark CSV input file.")
	parser.add_argument("--output-dir", type=Path, default=DEFAULT_OUTPUT_DIR, help="Directory for plots and summary outputs.")
	parser.add_argument("--metric", default=DEFAULT_METRIC, help="Throughput metric to normalize, defaults to overall_throughput_mb_s.")
	parser.add_argument(
		"--distribution-weights",
		default=None,
		help="Optional weights like random=2,ascending=1,descending=1. Defaults to equal weight across workloads.",
	)
	parser.add_argument(
		"--size-weights",
		default=None,
		help="Optional weights like 16=1,400=1. Defaults to equal weight across sizes.",
	)
	return parser


def load_benchmark_data(input_path: Path) -> pd.DataFrame:
	frame = pd.read_csv(input_path)
	required_columns = {
		"size_mb",
		"distribution",
		"thread_count",
		"fan_in_factor",
		"phase1_seconds",
		"phase2_seconds",
		"total_seconds",
		"phase1_throughput_mb_s",
		"phase2_throughput_mb_s",
		"overall_throughput_mb_s",
	}
	missing = required_columns.difference(frame.columns)
	if missing:
		raise ValueError(f"missing expected CSV columns: {sorted(missing)}")
	return frame


def compute_normalized_scores(frame: pd.DataFrame, metric: str) -> pd.DataFrame:
	if metric not in frame.columns:
		raise ValueError(f"metric '{metric}' not found in CSV")

	workload_columns = ["size_mb", "distribution"]
	frame = frame.copy()
	frame["workload_id"] = frame[workload_columns].astype(str).agg("|".join, axis=1)
	frame["workload_peak"] = frame.groupby("workload_id")[metric].transform("max")
	frame["relative_efficiency"] = frame[metric] / frame["workload_peak"]
	return frame


def apply_workload_weights(frame: pd.DataFrame, distribution_weights: Dict[str, float], size_weights: Dict[str, float]) -> pd.DataFrame:
	frame = frame.copy()
	frame["distribution_weight"] = frame["distribution"].map(lambda value: distribution_weights.get(value, 1.0))
	frame["size_weight"] = frame["size_mb"].map(lambda value: size_weights.get(str(value), 1.0))
	frame["workload_weight"] = frame["distribution_weight"] * frame["size_weight"]
	return frame


def summarize_configurations(frame: pd.DataFrame) -> pd.DataFrame:
	grouped = (
		frame.groupby(["thread_count", "fan_in_factor"], as_index=False)
		.agg(
			avg_efficiency_index=("relative_efficiency", "mean"),
			weighted_efficiency_index=("relative_efficiency", lambda values: values.mul(frame.loc[values.index, "workload_weight"]).sum()
				/ frame.loc[values.index, "workload_weight"].sum()),
			avg_total_seconds=("total_seconds", "mean"),
			avg_throughput=("overall_throughput_mb_s", "mean"),
		)
		.sort_values(["weighted_efficiency_index", "avg_efficiency_index", "avg_throughput"], ascending=[False, False, False])
	)
	grouped["avg_efficiency_pct"] = grouped["avg_efficiency_index"] * 100.0
	grouped["weighted_efficiency_pct"] = grouped["weighted_efficiency_index"] * 100.0
	return grouped


def save_summary_tables(frame: pd.DataFrame, summary: pd.DataFrame, output_dir: Path) -> None:
	output_dir.mkdir(parents=True, exist_ok=True)
	frame.to_csv(output_dir / "normalized_rows.csv", index=False)
	summary.to_csv(output_dir / "configuration_ranking.csv", index=False)


def plot_throughput_vs_thread(frame: pd.DataFrame, output_dir: Path) -> None:
	sns.set_theme(style="whitegrid")
	for distribution in sorted(frame["distribution"].unique()):
		subset = frame[frame["distribution"] == distribution]
		fig, ax = plt.subplots(figsize=(10, 6))
		sns.lineplot(
			data=subset,
			x="thread_count",
			y="overall_throughput_mb_s",
			hue="size_mb",
			marker="o",
			ax=ax,
		)
		ax.set_title(f"Throughput vs Thread Count ({distribution})")
		ax.set_xlabel("Thread Count")
		ax.set_ylabel("Throughput (MB/s)")
		fig.tight_layout()
		fig.savefig(output_dir / f"throughput_vs_thread_{distribution}.png", dpi=200)
		plt.close(fig)


def plot_total_time_vs_fanin(frame: pd.DataFrame, output_dir: Path) -> None:
	sns.set_theme(style="whitegrid")
	for distribution in sorted(frame["distribution"].unique()):
		subset = frame[frame["distribution"] == distribution]
		fig, ax = plt.subplots(figsize=(10, 6))
		sns.lineplot(
			data=subset,
			x="fan_in_factor",
			y="total_seconds",
			hue="size_mb",
			style="thread_count",
			marker="o",
			ax=ax,
		)
		ax.set_title(f"Total Sort Time vs Fan-In ({distribution})")
		ax.set_xlabel("Fan-In Factor")
		ax.set_ylabel("Total Time (s)")
		fig.tight_layout()
		fig.savefig(output_dir / f"total_time_vs_fanin_{distribution}.png", dpi=200)
		plt.close(fig)


def plot_distribution_comparison(frame: pd.DataFrame, output_dir: Path) -> None:
	fig, ax = plt.subplots(figsize=(12, 6))
	summary = (
		frame.groupby(["distribution", "size_mb"], as_index=False)
		.agg(avg_total_seconds=("total_seconds", "mean"), avg_phase1_seconds=("phase1_seconds", "mean"), avg_phase2_seconds=("phase2_seconds", "mean"))
	)
	sns.barplot(data=summary, x="distribution", y="avg_total_seconds", hue="size_mb", ax=ax)
	ax.set_title("Average Total Time by Distribution")
	ax.set_xlabel("Distribution")
	ax.set_ylabel("Average Total Time (s)")
	fig.tight_layout()
	fig.savefig(output_dir / "distribution_vs_time_total.png", dpi=200)
	plt.close(fig)

	fig, ax = plt.subplots(figsize=(12, 6))
	long_frame = summary.melt(id_vars=["distribution", "size_mb"], value_vars=["avg_phase1_seconds", "avg_phase2_seconds"], var_name="phase", value_name="seconds")
	sns.barplot(data=long_frame, x="distribution", y="seconds", hue="phase", ax=ax)
	ax.set_title("Phase Breakdown by Distribution")
	ax.set_xlabel("Distribution")
	ax.set_ylabel("Average Seconds")
	fig.tight_layout()
	fig.savefig(output_dir / "distribution_phase_breakdown.png", dpi=200)
	plt.close(fig)


def plot_efficiency_heatmap(summary: pd.DataFrame, output_dir: Path) -> None:
	heatmap = summary.pivot(index="thread_count", columns="fan_in_factor", values="weighted_efficiency_pct")
	fig, ax = plt.subplots(figsize=(10, 6))
	sns.heatmap(heatmap, annot=True, fmt=".1f", cmap="viridis", ax=ax)
	ax.set_title("Weighted Efficiency Index by Configuration")
	ax.set_xlabel("Fan-In Factor")
	ax.set_ylabel("Thread Count")
	fig.tight_layout()
	fig.savefig(output_dir / "weighted_efficiency_heatmap.png", dpi=200)
	plt.close(fig)


def plot_size_specific_heatmaps(frame: pd.DataFrame, output_dir: Path) -> None:
	for size_mb in sorted(frame["size_mb"].unique()):
		subset = frame[frame["size_mb"] == size_mb]
		size_summary = (
			subset.groupby(["thread_count", "fan_in_factor"], as_index=False)
			.agg(weighted_efficiency_pct=("relative_efficiency", lambda values: values.mul(subset.loc[values.index, "workload_weight"]).sum()
				/ subset.loc[values.index, "workload_weight"].sum()))
		)
		heatmap = size_summary.pivot(index="thread_count", columns="fan_in_factor", values="weighted_efficiency_pct")
		fig, ax = plt.subplots(figsize=(10, 6))
		sns.heatmap(heatmap, annot=True, fmt=".1f", cmap="viridis", ax=ax)
		ax.set_title(f"Weighted Efficiency Heatmap ({size_mb} MB)")
		ax.set_xlabel("Fan-In Factor")
		ax.set_ylabel("Thread Count")
		fig.tight_layout()
		fig.savefig(output_dir / f"weighted_efficiency_heatmap_{size_mb}mb.png", dpi=200)
		plt.close(fig)

	if {16, 400}.issubset(set(frame["size_mb"].unique())):
		size_16 = frame[frame["size_mb"] == 16]
		size_400 = frame[frame["size_mb"] == 400]
		heat_16 = (
			size_16.groupby(["thread_count", "fan_in_factor"], as_index=False)
			.agg(weighted_efficiency_pct=("relative_efficiency", lambda values: values.mul(size_16.loc[values.index, "workload_weight"]).sum()
				/ size_16.loc[values.index, "workload_weight"].sum()))
			.pivot(index="thread_count", columns="fan_in_factor", values="weighted_efficiency_pct")
		)
		heat_400 = (
			size_400.groupby(["thread_count", "fan_in_factor"], as_index=False)
			.agg(weighted_efficiency_pct=("relative_efficiency", lambda values: values.mul(size_400.loc[values.index, "workload_weight"]).sum()
				/ size_400.loc[values.index, "workload_weight"].sum()))
			.pivot(index="thread_count", columns="fan_in_factor", values="weighted_efficiency_pct")
		)
		diff = heat_400 - heat_16
		fig, ax = plt.subplots(figsize=(10, 6))
		sns.heatmap(diff, annot=True, fmt="+.1f", cmap="coolwarm", center=0, ax=ax)
		ax.set_title("Weighted Efficiency Difference (400 MB - 16 MB)")
		ax.set_xlabel("Fan-In Factor")
		ax.set_ylabel("Thread Count")
		fig.tight_layout()
		fig.savefig(output_dir / "weighted_efficiency_heatmap_400mb_minus_16mb.png", dpi=200)
		plt.close(fig)


def main() -> None:
	args = build_parser().parse_args()
	distribution_weights = parse_weights(args.distribution_weights)
	size_weights = parse_weights(args.size_weights)

	frame = load_benchmark_data(args.input)
	frame = compute_normalized_scores(frame, args.metric)
	frame = apply_workload_weights(frame, distribution_weights, size_weights)
	summary = summarize_configurations(frame)

	args.output_dir.mkdir(parents=True, exist_ok=True)
	save_summary_tables(frame, summary, args.output_dir)
	plot_throughput_vs_thread(frame, args.output_dir)
	plot_total_time_vs_fanin(frame, args.output_dir)
	plot_distribution_comparison(frame, args.output_dir)
	plot_efficiency_heatmap(summary, args.output_dir)
	plot_size_specific_heatmaps(frame, args.output_dir)

	best = summary.iloc[0]
	print("Top configuration by weighted efficiency:")
	print(
		f"thread_count={int(best.thread_count)}, fan_in_factor={int(best.fan_in_factor)}, "
		f"weighted_efficiency={best.weighted_efficiency_pct:.2f}%"
	)
	print(f"Saved plots and CSV summaries in {args.output_dir}")


if __name__ == "__main__":
	main()