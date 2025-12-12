use std::time::Duration;

#[allow(dead_code)]
pub trait MediaEngine {
    fn play(
        &mut self,
        api_url: &str,
        token: &str,
        track_id: u64,
        duration_ms: u64,
        prefetched_cdn_url: Option<String>,
    ) -> Result<(), String>;

    fn seek(&mut self, position: Duration) -> Result<(), String>;
    fn stop(&mut self);
    fn set_volume(&mut self, volume: f32);
    fn is_finished(&self) -> bool;
    fn get_position(&self) -> Duration;
    fn get_duration(&self) -> Option<Duration>;
}
