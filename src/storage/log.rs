use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
};

use anyhow::{Context, Result};
use chrono::{DateTime, SecondsFormat, Utc};

/// Formats a time as one log line, e.g. `2026-10-06T08:03:09.123456789Z`.
/// Keeps every sub second digit, so reading the line back gives same time
fn to_line(time: DateTime<Utc>) -> String {
    time.to_rfc3339_opts(SecondsFormat::AutoSi, true)
}

/// Appends the current local time as a new line, creating the file if missing.
/// Returns the entry that was written.
///
/// # Errors
/// Returns an error if the file can't be opened or written.
pub fn add_entry(file_name: &str) -> Result<DateTime<Utc>> {
    let entry = Utc::now();
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(file_name)
        .with_context(|| format!("failed to open {file_name}"))?;

    writeln!(file, "{}", to_line(entry))
        .with_context(|| format!("failed to write to {file_name}"))?;
    Ok(entry)
}

/// Empties the log file without deleting it.
///
/// # Errors
/// Returns an error if the file can't be written.
pub fn clear_all(file_name: &str) -> Result<()> {
    fs::write(file_name, "").with_context(|| format!("failed to clear the log in {file_name}"))?;
    Ok(())
}

/// Returns the full log contents. A missing file counts as an empty log.
///
/// # Errors
/// Returns an error if the file exists but can't be read, or a line isn't a
/// valid RFC 3339 time.
pub fn show_log(file_name: &str) -> Result<Vec<DateTime<Utc>>> {
    let contents = match fs::read_to_string(file_name) {
        Ok(contents) => contents,
        // No log file yet means an empty log, not an error
        Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e).with_context(|| format!("failed to read {file_name}")),
    };

    contents
        .lines()
        .map(|line| {
            line.parse::<DateTime<Utc>>()
                .with_context(|| format!("failed to parse entry {line:?} in {file_name}"))
        })
        .collect()
}

/// Returns the newest entry, or `None` if the log is empty.
///
/// # Errors
/// Returns an error if the log can't be read
pub fn show_last(file_name: &str) -> Result<Option<DateTime<Utc>>> {
    Ok(show_log(file_name)?.pop())
}

/// Removes the newest entry. Does nothing if the log is empty.
///
/// # Errors
/// Returns an error if the log can't be read or written.
pub fn clear_last(file_name: &str) -> Result<()> {
    let mut entries = show_log(file_name)?;
    entries.pop();

    let contents: String = entries.into_iter().map(|t| to_line(t) + "\n").collect();
    fs::write(file_name, contents)
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

    #[track_caller]
    fn time(rfc3339: &str) -> DateTime<Utc> {
        rfc3339.parse().expect("test time should be valid RFC 3339")
    }

    #[test]
    fn add_entry_writes_rfc3339_utc_line() {
        let (_dir, path) = temp_log();

        let entry = add_entry(&path).expect("entry should be written");

        let contents = fs::read_to_string(&path).expect("log file should be readable");
        assert!(
            contents.ends_with("Z\n"),
            "line should be in UTC: {contents:?}"
        );
        let written = DateTime::parse_from_rfc3339(contents.trim_end())
            .expect("line should parse as RFC 3339");
        assert_eq!(written, entry);
    }

    #[test]
    fn show_log_returns_empty_when_file_missing() {
        let (_dir, path) = temp_log();

        let entries = show_log(&path).expect("missing log should be read as empty");

        assert!(entries.is_empty(), "expected no entries, got {entries:?}");
    }

    #[test]
    fn show_log_errors_when_path_is_unreadable() {
        let (_dir, path) = temp_log();
        fs::create_dir(&path).expect("directory should be creatable at log path");

        let result = show_log(&path);

        assert!(
            result.is_err(),
            "reading a directory should be an error, not an empty log"
        );
    }

    #[test]
    fn show_log_errors_on_line_that_is_not_a_time() {
        let (_dir, path) = temp_log();
        fs::write(&path, "Mon, Oct 06 2026 14:03:09\n").expect("log should be seedable");

        let result = show_log(&path);

        assert!(result.is_err(), "an old-format line should be an error");
    }

    #[test]
    fn add_entry_appends_without_overwriting() {
        let (_dir, path) = temp_log();

        let first = add_entry(&path).expect("first line should be written");
        let second = add_entry(&path).expect("second line should be written");

        let entries = show_log(&path).expect("log should be readable");
        assert_eq!(entries, vec![first, second]);
    }

    #[test]
    fn show_last_returns_final_line() {
        let (_dir, path) = temp_log();
        fs::write(&path, "2026-10-01T08:00:00Z\n2026-10-02T09:30:00Z\n")
            .expect("log should be seedable");

        let last = show_last(&path).expect("last entry should be readable");

        assert_eq!(last, Some(time("2026-10-02T09:30:00Z")));
    }

    #[test]
    fn show_last_is_none_when_log_missing() {
        let (_dir, path) = temp_log();

        let last = show_last(&path).expect("last entry should be readable");

        assert_eq!(last, None);
    }

    #[test]
    fn clear_last_keeps_earlier_lines() {
        let (_dir, path) = temp_log();
        fs::write(
            &path,
            "2026-10-01T08:00:00Z\n2026-10-02T09:30:00Z\n2026-10-03T10:45:00Z\n",
        )
        .expect("log should be seedable");

        clear_last(&path).expect("last entry should be clearable");

        let contents = fs::read_to_string(&path).expect("log file should be readable");
        assert_eq!(contents, "2026-10-01T08:00:00Z\n2026-10-02T09:30:00Z\n");
    }

    #[test]
    fn clear_last_on_single_line_leaves_empty_file() {
        let (_dir, path) = temp_log();
        fs::write(&path, "2026-10-01T08:00:00Z\n").expect("log should be seedable");

        clear_last(&path).expect("last entry should be clearable");

        let contents = fs::read_to_string(&path).expect("log file should be readable");
        assert_eq!(contents, "");
    }

    #[test]
    fn clear_all_empties_file() {
        let (_dir, path) = temp_log();
        fs::write(&path, "first\nsecond\n").expect("log should be seedable");

        clear_all(&path).expect("log should be clearable");

        let contents = fs::read_to_string(&path).expect("log file should be readable");
        assert_eq!(contents, "");
    }
}
