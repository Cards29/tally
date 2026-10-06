use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};

/// Returns the current local time formatted as `Mon, Oct 06 2026 14:03:09`.
fn current_time() -> String {
    let local_time: DateTime<Local> = Local::now();

    local_time.format("%a, %b %d %Y %H:%M:%S").to_string()
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
    fs::write(file_name, new_contents)
        .with_context(|| format!("failed to write {file_name} after clearing last line"))
}

#[cfg(test)]
mod tests {
    use std::fs;

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
    fn read_log_returns_empty_when_file_missing() {
        let (_dir, path) = temp_log();
        let contents = read_log(&path).expect("missing log should be read as empty");

        assert_eq!(contents, "");
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
    fn clear_last_entry_on_missing_file_creates_empty_file() {
        let (_dir, path) = temp_log();

        clear_last_entry(&path).expect("clearing last of a missing log should succeed");

        let contents = fs::read_to_string(&path).expect("log file should now exist");
        assert_eq!(contents, "");
    }

    #[test]
    fn clear_last_entry_on_empty_file_stays_empty() {
        let (_dir, path) = temp_log();
        fs::write(&path, "").expect("log should be seedable");

        clear_last_entry(&path).expect("clearing last of an empty log should succeed");

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

    #[test]
    fn clear_log_on_missing_file_creates_empty_file() {
        let (_dir, path) = temp_log();
        clear_log(&path).expect("clearing a missing log should succeed");

        let contents = fs::read_to_string(&path).expect("log file should now exist");
        assert_eq!(contents, "");
    }

    #[test]
    fn clear_log_twice_succeeds() {
        let (_dir, path) = temp_log();
        fs::write(&path, "first\n").expect("log should be seedable");

        clear_log(&path).expect("first clear should succeed");
        clear_log(&path).expect("second clear should succeed");

        let contents = fs::read_to_string(&path).expect("log file should be readable");
        assert_eq!(contents, "");
    }

    #[test]
    fn log_time_after_clear_starts_fresh() {
        let (_dir, path) = temp_log();
        fs::write(&path, "old\n").expect("log should be seedable");

        clear_log(&path).expect("log should be clearable");
        let entry = log_time(&path).expect("entry should be written after clear");

        let contents = fs::read_to_string(&path).expect("log file should be readable");
        assert_eq!(contents, format!("{entry}\n"));
    }

    #[test]
    fn clear_log_fails_when_parent_dir_missing() {
        let (dir, _) = temp_log();
        let path = dir
            .path()
            .join("missing")
            .join("test.log")
            .to_str()
            .expect("temp path should be valid utf-8")
            .to_string();

        let err = clear_log(&path).expect_err("clearing in a missing dir should fail");

        assert_eq!(err.to_string(), "failed to clear the log");
    }
}
