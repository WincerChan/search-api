use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref DATETIME_RE: Regex =
        Regex::new(r"([\d]{4})-([\d]{2})-([\d]{2})T[\d]{2}:[\d]{2}:[\d]{2}\+[\d]{2}:00").unwrap();
}

pub fn str_to_datetime(time_str: &str) -> i64 {
    if !DATETIME_RE.is_match(time_str) {
        return 0;
    }
    DATETIME_RE.captures_iter(time_str);
    println!("output i {:?}", DATETIME_RE.captures(time_str));
    0
}

#[test]
fn test_datetime_parse() {
    str_to_datetime("2022-09-15T23:45:08+08:00");
}

// pub fn str_to_date(date_str: &str) {
//     Regex::new(r"[\d]{4}-[\d]{2}-[\d]{2}")
// }
