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
pub fn clean_filename(original: String) -> Option<String> {
    if original.chars().any(|c| !c.is_ascii()) {
        return None;
    }

    // split into name and extension
    // extension is everything after the last dot
    let mut parts = original.split('.');

    let extension = parts.next_back().unwrap_or("");
    if extension.chars().any(|c| !c.is_ascii_alphanumeric() && c != '-' && c != '_') {
        return None;
    }

    let name = parts.collect::<Vec<&str>>().join(".");

    let clean: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' {
                c
            } else {
                ' '
            }
        }).collect();    
    
    let mut result = clean.split_whitespace().collect::<Vec<&str>>().join("_");

    if extension.len() > 0 {
        result.push('.');
        result.push_str(extension);
    }

    Some(result)
}
