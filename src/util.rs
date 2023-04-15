pub fn constant_time_compare(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }

    a.bytes().zip(b.bytes())
        .fold(0, |acc, (a, b)| acc | (a ^ b) ) == 0
}