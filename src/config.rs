pub const U64_BYTES: usize = 8;
pub const READ_BUFFER_BYTES: usize = 16 * 1024;
pub const MEMORY_LIMIT_BYTES: usize = 20_000_000;
pub const DEFAULT_THREAD_COUNT: usize = 8;
pub const DEFAULT_FAN_IN_FACTOR: usize = 4;
pub const PREVIEW_COUNT: usize = 10;

pub fn max_chunk_values() -> usize {
    MEMORY_LIMIT_BYTES / U64_BYTES
}