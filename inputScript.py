from __future__ import annotations

import argparse
import random
import struct
from pathlib import Path


U64_SIZE = 8
BLOCK_SIZE_BYTES = 1 << 20
DEFAULT_SIZE_MB = 100
DEFAULT_PREVIEW_COUNT = 10
DEFAULT_SEED = None


def generate_u64_file(output_path: Path, size_bytes: int, seed: int | None = DEFAULT_SEED) -> None:
	if size_bytes % U64_SIZE != 0:
		raise ValueError("size_bytes must be a multiple of 8")

	value_count = size_bytes // U64_SIZE
	values_per_block = BLOCK_SIZE_BYTES // U64_SIZE
	rng = random.Random(seed)
	packer = struct.Struct("<Q")

	output_path.parent.mkdir(parents=True, exist_ok=True)

	with output_path.open("wb") as file_handle:
		remaining = value_count
		while remaining > 0:
			block_value_count = min(values_per_block, remaining)
			buffer = bytearray(block_value_count * U64_SIZE)

			for index in range(block_value_count):
				packer.pack_into(buffer, index * U64_SIZE, rng.getrandbits(64))

			file_handle.write(buffer)
			remaining -= block_value_count


def preview_u64_file(file_path: Path, count: int = DEFAULT_PREVIEW_COUNT) -> tuple[list[int], list[int]]:
	file_size = file_path.stat().st_size
	if file_size % U64_SIZE != 0:
		raise ValueError("file size must be a multiple of 8")

	total_values = file_size // U64_SIZE
	if total_values == 0:
		return [], []

	read_count = min(count, total_values)
	unpack = struct.Struct("<Q").unpack

	with file_path.open("rb") as file_handle:
		first_values = [
			unpack(file_handle.read(U64_SIZE))[0]
			for _ in range(read_count)
		]

		tail_offset = max(0, total_values - read_count) * U64_SIZE
		file_handle.seek(tail_offset)
		last_values = [
			unpack(file_handle.read(U64_SIZE))[0]
			for _ in range(read_count)
		]

	return first_values, last_values


def verify_sorted_u64_file(file_path: Path) -> tuple[bool, int | None, int | None]:
	file_size = file_path.stat().st_size
	if file_size % U64_SIZE != 0:
		raise ValueError("file size must be a multiple of 8")

	unpack = struct.Struct("<Q").unpack
	previous_value: int | None = None
	index = 0

	with file_path.open("rb") as file_handle:
		while True:
			chunk = file_handle.read(1 << 20)
			if not chunk:
				break

			if len(chunk) % U64_SIZE != 0:
				raise ValueError("file contains a partial u64 record")

			for offset in range(0, len(chunk), U64_SIZE):
				current_value = unpack(chunk[offset : offset + U64_SIZE])[0]
				if previous_value is not None and current_value < previous_value:
					return False, index - 1, index
				previous_value = current_value
				index += 1

	return True, None, None


def format_values(values: list[int]) -> str:
	return ", ".join(str(value) for value in values)


def build_parser() -> argparse.ArgumentParser:
	parser = argparse.ArgumentParser(description="Generate and preview a binary u64 file.")
	parser.add_argument(
		"--output",
		type=Path,
		default=Path("input.bin"),
		help="Path to the binary output file.",
	)
	parser.add_argument(
		"--size-mb",
		type=int,
		default=DEFAULT_SIZE_MB,
		help="Target file size in mebibytes.",
	)
	parser.add_argument(
		"--seed",
		type=int,
		default=DEFAULT_SEED,
		help="Optional RNG seed for repeatable output.",
	)
	parser.add_argument(
		"--preview-count",
		type=int,
		default=DEFAULT_PREVIEW_COUNT,
		help="Number of u64 values to preview from the start and end of the file.",
	)
	parser.add_argument(
		"--preview-only",
		action="store_true",
		help="Skip generation and only preview an existing binary file.",
	)
	parser.add_argument(
		"--verify-sorted",
		action=argparse.BooleanOptionalAction,
		default=True,
		help="Sequentially verify whether the binary file is sorted.",
	)
	return parser


def main() -> None:
	parser = build_parser()
	args = parser.parse_args()

	file_size_bytes = args.size_mb * 1024 * 1024

	if not args.preview_only:
		generate_u64_file(args.output, file_size_bytes, seed=args.seed)

	first_values, last_values = preview_u64_file(args.output, count=args.preview_count)
	sorted_ok = None
	first_bad_index = None
	second_bad_index = None
	if args.verify_sorted:
		sorted_ok, first_bad_index, second_bad_index = verify_sorted_u64_file(args.output)

	print(f"file: {args.output}")
	print(f"size bytes: {args.output.stat().st_size}")
	print(f"u64 count: {args.output.stat().st_size // U64_SIZE}")
	print(f"first {len(first_values)}: {format_values(first_values)}")
	print(f"last {len(last_values)}: {format_values(last_values)}")
	if args.verify_sorted:
		if sorted_ok:
			print("sorted: yes")
		else:
			print(
				f"sorted: no (value at index {first_bad_index} is greater than value at index {second_bad_index})"
			)


if __name__ == "__main__":
	main()
