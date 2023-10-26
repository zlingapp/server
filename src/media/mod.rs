use lazy_static::lazy_static;
use regex::Regex;

pub mod routes;

lazy_static! {
    pub static ref FILENAME_REGEX: Regex = Regex::new(r"^[a-zA-Z0-9_.-]+$").unwrap();
}

pub mod util;
