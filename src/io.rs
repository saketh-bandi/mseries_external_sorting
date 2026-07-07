use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;

pub fn open_run_writer(path: &Path) -> io::Result<BufWriter<File>> {
	let file = File::create(path)?;
	Ok(BufWriter::new(file))
}

pub fn write_u64_run(writer: &mut BufWriter<File>, values: &[u64]) -> io::Result<()> {
	for value in values {
		writer.write_all(&value.to_le_bytes())?;
	}
	writer.flush()
}