use chrono::{Datelike, FixedOffset, Timelike, Utc};
use std::fs;

const FILE_PATH: &str = "src/dates.log";
fn main() {
    let mut contents =
        fs::read_to_string(FILE_PATH).expect("Should have been able to read the file");

    let offset =
        FixedOffset::east_opt(6 * 60 * 60).expect("Should calculate the offset by 6 hours");
    let now = Utc::now().with_timezone(&offset);
    let (is_pm, hour) = now.hour12();
    let current_hour = hour.to_string();
    let current_minute = now.minute().to_string();
    let current_second = now.second().to_string();

    let current_day = now.day().to_string();
    let current_month = now.month().to_string();
    let current_year = now.year().to_string();
    let current_weekday = now.weekday().to_string();

    contents += &current_day[..];
    contents += "-";
    contents += &current_month[..];
    contents += "-";
    contents += &current_year[..];
    contents += " ";
    contents += &current_weekday[..];
    contents += " ";
    contents += &current_hour[..];
    contents += ":";
    contents += &current_minute[..];
    contents += ":";
    contents += &current_second[..];
    contents += " ";
    if is_pm {
        contents += "PM";
    } else {
        contents += "AM";
    }
    contents += "\n";

    fs::write(FILE_PATH, contents).expect("Should write on the file");
}
