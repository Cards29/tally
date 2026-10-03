use chrono::{DateTime, Local};
use std::fs;

const FILE_PATH: &str = "src/dates.log";
fn main() {
    let mut contents =
        fs::read_to_string(FILE_PATH).expect("Should have been able to read the file");

    let local_time: DateTime<Local> = Local::now();
    println!("{}", local_time);

    contents += &local_time.to_string()[..];
    contents += "\n";

    fs::write(FILE_PATH, contents).expect("Should write on the file");
}
