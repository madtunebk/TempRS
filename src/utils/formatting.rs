/// Format seconds into MM:SS format
pub fn format_duration(seconds: f32) -> String {
    let total_seconds = seconds as i32;
    let minutes = total_seconds / 60;
    let secs = total_seconds % 60;
    format!("{:02}:{:02}", minutes, secs)
}
