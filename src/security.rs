/*

   security.rs

   This file is responsible for the general maintenance of security of the application.
   Put functions that are related to validating user input, checking for security vulnerabilities, etc.

   Cryptographic functions should be put in crypto.rs.

*/

use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref RESOURCE_REGEX: Regex =
        Regex::new(r"^/media/[a-zA-Z0-9_-]+/[a-zA-Z0-9_.-]+$").unwrap();
}

/// Ensures that a resource url is from the zling media api.
pub fn validate_resource_origin(url: &str) -> bool {
    RESOURCE_REGEX.is_match(url)
}
