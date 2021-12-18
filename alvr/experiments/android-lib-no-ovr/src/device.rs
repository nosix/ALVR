pub struct Device {
    pub name: String,
    pub recommended_eye_width: u32,
    pub recommended_eye_height: u32,
    pub available_refresh_rates: Vec<f32>,
    pub preferred_refresh_rate: f32
}
