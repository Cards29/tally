use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
};

use anyhow::{Context, Result};
use chrono::{DateTime, Local};

const TIME_FORMAT: &str = "%a, %b %d %Y %H:%M:%S";

/// Returns the current local time formatted as `Mon, Oct 06 2026 14:03:09`.
fn current_time() -> String {
    let local_time: DateTime<Local> = Local::now();

    local_time.format(TIME_FORMAT).to_string()
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
        .with_context(|| format!("failed to open {file_name}"))?;

    writeln!(file, "{entry}").with_context(|| format!("failed to write to {file_name}"))?;
    Ok(entry)
}

/// Empties the log file without deleting it.
///
/// # Errors
/// Returns an error if the file can't be written.
pub fn clear_log(file_name: &str) -> Result<()> {
    fs::write(file_name, "").with_context(|| format!("failed to clear the log in {file_name}"))?;
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

/// Splits the log into (everything before the last line, the last line).
fn split_last_line(contents: &str) -> (&str, &str) {
    let trimmed = contents.trim_end_matches('\n');
    match trimmed.rfind('\n') {
        Some(i) => (&trimmed[..=i], &trimmed[i + 1..]),
        None => ("", trimmed),
    }
}

/// Returns the last line of the log, or `""` if the log is empty.
///
/// # Errors
/// Returns an error if the log can't be read.
pub fn last_entry(file_name: &str) -> Result<String> {
    let contents = read_log(file_name)?;
    Ok(split_last_line(&contents).1.to_string())
}

/// Removes the last line of the log. Does nothing if the log is empty.
///
/// # Errors
/// Returns an error if the log can't be read or written.
pub fn clear_last_entry(file_name: &str) -> Result<()> {
    let contents = read_log(file_name)?;
    let new_contents = split_last_line(&contents).0;

    fs::write(file_name, new_contents)
        .with_context(|| format!("failed to write {file_name} after clearing last line"))
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    // Returns a temp dir plus a log path inside it. The log file itself is not
    // created, so each test decides whether it exists. Keep the TempDir alive:
    // dropping it deletes the directory.
    #[track_caller]
    fn temp_log() -> (TempDir, String) {
        let dir = TempDir::new().expect("temp dir should be creatable");
        let path = dir
            .path()
            .join("test.log")
            .to_str()
            .expect("temp path should be valid utf-8")
            .to_string();
        (dir, path)
    }

    #[test]
    fn current_time_matches_log_format() {
        let entry = current_time();

        chrono::NaiveDateTime::parse_from_str(&entry, "%a, %b %d %Y %H:%M:%S")
            .expect("entry should parse with the log format");
    }

    #[test]
    fn read_log_returns_empty_when_file_missing() {
        let (_dir, path) = temp_log();

        let contents = read_log(&path).expect("missing log should be read as empty");

        assert_eq!(contents, "");
    }

    #[test]
    fn read_log_errors_when_path_is_unreadable() {
        let (_dir, path) = temp_log();
        fs::create_dir(&path).expect("directory should be creatable at log path");

        let result = read_log(&path);

        assert!(
            result.is_err(),
            "reading a directory should be an error, not an empty log"
        );
    }

    #[test]
    fn log_time_appends_without_overwriting() {
        let (_dir, path) = temp_log();

        let first = log_time(&path).expect("first line should be written");
        let second = log_time(&path).expect("second line should be written");

        let contents = fs::read_to_string(&path).expect("log file should be readable");
        assert_eq!(contents, format!("{first}\n{second}\n"));
    }

    #[test]
    fn last_entry_returns_final_line() {
        let (_dir, path) = temp_log();
        fs::write(&path, "first\nsecond\n").expect("log should be seedable");

        let contents = last_entry(&path).expect("last entry should be readable");

        assert_eq!(contents, "second");
    }

    #[test]
    fn last_entry_is_empty_when_log_missing() {
        let (_dir, path) = temp_log();

        let contents = last_entry(&path).expect("last entry should be readable");

        assert_eq!(contents, "");
    }

    #[test]
    fn clear_last_entry_keeps_earlier_lines() {
        let (_dir, path) = temp_log();
        fs::write(&path, "first\nsecond\nthird\n").expect("log should be seedable");

        clear_last_entry(&path).expect("last entry should be clearable");

        let contents = fs::read_to_string(&path).expect("log file should be readable");
        assert_eq!(contents, "first\nsecond\n");
    }

    #[test]
    fn clear_last_entry_on_single_line_leaves_empty_file() {
        let (_dir, path) = temp_log();
        fs::write(&path, "only\n").expect("log should be seedable");

        clear_last_entry(&path).expect("last entry should be clearable");

        let contents = fs::read_to_string(&path).expect("log file should be readable");
        assert_eq!(contents, "");
    }

    #[test]
    fn clear_log_empties_file() {
        let (_dir, path) = temp_log();
        fs::write(&path, "first\nsecond\n").expect("log should be seedable");

        clear_log(&path).expect("log should be clearable");

        let contents = fs::read_to_string(&path).expect("log file should be readable");
        assert_eq!(contents, "");
    }
}
