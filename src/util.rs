use std::fmt::Display;

use serde::Serializer;

pub fn constant_time_compare(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }

    a.bytes().zip(b.bytes())
        .fold(0, |acc, (a, b)| acc | (a ^ b) ) == 0
}

pub fn use_display<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    T: Display,
    S: Serializer
{
    serializer.collect_str(value)
}