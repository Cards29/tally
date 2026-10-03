use chrono::{DateTime, Local};
use std::fs;

const FILE_PATH: &str = "src/dates.log";

fn current_time() -> String {
    let local_time: DateTime<Local> = Local::now();

    let local_time = local_time.format("%a, %b %d %Y %I:%M:%S %p\n").to_string();
    println!("{}", local_time);

    let local_time = local_time.to_string();
    local_time
}

fn log_time() {
    let mut contents =
        fs::read_to_string(FILE_PATH).expect("Should have been able to read the file");

    // contents += &current_time()[..];
    contents.push_str(&current_time());

    fs::write(FILE_PATH, contents).expect("Should write on the file");
}

fn clear_log() {
    fs::write(FILE_PATH, "").expect("Should clear the file");
}

fn main() {
    log_time();
}
