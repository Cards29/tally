use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};

/// Returns the current local time formatted as `Mon, Oct 06 2026 14:03:09`.
fn current_time() -> String {
    let local_time: DateTime<Local> = Local::now();
    let local_time = local_time.format("%a, %b %d %Y %H:%M:%S").to_string();
    local_time
}

/// Appends the current local time as a new line, creating the file if missing.
/// Returns the entry that was written.
///
/// # Errors
/// Returns an error if the file can't be opened or written.
pub fn log_time(file_name: &str) -> Result<String> {
    let entry = current_time();
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(file_name)
        .with_context(|| format!("failed to open {}", file_name))?;

    writeln!(file, "{}", entry).with_context(|| format!("failed to write to {}", file_name))?;
    Ok(entry)
}

/// Empties the log file without deleting it.
///
/// # Errors
/// Returns an error if the file can't be written.
pub fn clear_log(file_name: &str) -> Result<()> {
    fs::write(file_name, "").with_context(|| "failed to clear the log")?;
    Ok(())
}

/// Returns the full log contents. A missing file counts as an empty log.
///
/// # Errors
/// Returns an error if the file exists but can't be read.
pub fn read_log(file_name: &str) -> Result<String> {
    match fs::read_to_string(file_name) {
        Ok(contents) => Ok(contents),
        // No log file yet means an empty log, not an error
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(e).with_context(|| format!("failed to read {file_name}")),
    }
}

/// Returns the last line of the log, or `""` if the log is empty.
///
/// # Errors
/// Returns an error if the log can't be read.
pub fn last_entry(file_name: &str) -> Result<String> {
    let contents = read_log(file_name)?;
    Ok(contents.lines().last().unwrap_or("").to_string())
}

/// Removes the last line of the log. Does nothing if the log is empty.
///
/// # Errors
/// Returns an error if the log can't be read or written.
pub fn clear_last_entry(file_name: &str) -> Result<()> {
    let contents = read_log(file_name)?;
    let trimmed = contents.trim_end_matches("\n");
    let new_contents = match trimmed.rfind("\n") {
        Some(i) => &trimmed[..=i],
        None => "",
    };
    fs::write(file_name, new_contents).with_context(|| format!("failed to write {file_name}"))
}
