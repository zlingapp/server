use nanoid::nanoid;

/// Takes a filename and returns a cleaned version of it.
///
/// The returned filename will only contain ASCII alphanumeric characters, dots and dashes.
/// Extension must be ASCII alphanumeric characters or dashes, otherwise `None` is returned.
///
/// Examples:
/// "good_file.txt" -> "good_file.txt"
/// "Program SRC Version [1.2.3 Beta U H5 Pre 4].rar" -> Program_SRC_Version_1.2.3_Beta_U_H5_Pre_4.rar
/// "mosqit.....mp4" -> "mosqit.....mp4"
/// "file.&%&#^$" -> None
/// "Bad_Apple_played_on_Rocket_League (1).mp4" -> "Bad_Apple_played_on_Rocket_League_1.mp4"
/// "中国人.exe" -> None
pub fn clean_filename(mut original: String) -> Option<String> {
    if original.chars().any(|c| !c.is_ascii()) {
        return None;
    }

    // trim trailing dots
    while original.ends_with('.') {
        original.pop();
    }

    // split into name and extension
    // extension is everything after the last dot
    let mut parts = original.split('.').collect::<Vec<&str>>();

    // this entire part is needed because we need to handle the name
    let mut extension = "";
    if parts.len() > 1 {
        extension = parts.last().unwrap_or(&"");

        if extension
            .chars()
            .any(|c| !c.is_ascii_alphanumeric() && c != '-' && c != '_')
        {
            // extension is not valid, so we ignore it
            extension = "";
        } else {
            // extension is valid, so we remove it from the parts
            parts.pop();
        }
    }

    let name = parts.join(".");
    let mut result;

    if !name.contains(char::is_alphanumeric) {
        // the entire name is made up of weird characters, which is bad
        // make it a random string
        result = format!("file-{}", nanoid!(6));
    } else {
        let clean: String = name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '.' || c == '-' {
                    c
                } else {
                    ' '
                }
            })
            .collect();

        // trim repeated whitespaces and replace them with a single underscore
        result = clean.split_whitespace().collect::<Vec<&str>>().join("_");
    }

    // if we have a valid extension, append it
    if extension.len() > 0 {
        result.push('.');
        result.push_str(extension);
    }

    // trim trailing dots... again...
    while result.ends_with('.') {
        result.pop();
    }

    Some(result)
}
