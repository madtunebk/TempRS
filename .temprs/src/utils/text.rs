// No direct egui text types needed here

#[allow(dead_code)]
pub fn current_prefix(text: &str, caret_char: usize) -> String {
    let mut idx = caret_char.min(text.chars().count());
    let chars: Vec<char> = text.chars().collect();
    while idx > 0 {
        let c = chars[idx - 1];
        if c.is_alphanumeric() || c == '_' || c == '@' {
            idx -= 1;
        } else {
            break;
        }
    }
    chars[idx..caret_char.min(chars.len())].iter().collect()
}

#[allow(dead_code)]
pub fn byte_index_from_char_index(s: &str, char_index: usize) -> usize {
    if char_index == 0 {
        return 0;
    }
    for (count, (i, _)) in s.char_indices().enumerate() {
        if count == char_index {
            return i;
        }
    }
    s.len()
}

#[allow(dead_code)]
pub fn apply_completion(target: &mut String, caret_char: usize, word: &str) -> usize {
    let chars: Vec<char> = target.chars().collect();
    let mut start = caret_char.min(chars.len());
    while start > 0 {
        let c = chars[start - 1];
        if c.is_alphanumeric() || c == '_' || c == '@' {
            start -= 1;
        } else {
            break;
        }
    }
    let start_b = byte_index_from_char_index(target, start);
    let end_b = byte_index_from_char_index(target, caret_char.min(chars.len()));
    if start_b <= end_b && end_b <= target.len() {
        target.replace_range(start_b..end_b, word);
    }
    start + word.chars().count()
}
