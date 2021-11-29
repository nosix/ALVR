pub struct Device {
    name: String,
    headset: Headset
}

pub struct Headset {
    recommended_eye_width: u32,
    recommended_eye_height: u32,
    available_refresh_rates: Vec<f32>,
    preferred_refresh_rate: f32
}

impl Device {
    pub fn new(device_name: &str) -> Self {
        let available_refresh_rates = vec![60.];
        let preferred_refresh_rate = available_refresh_rates.last().cloned().unwrap_or(60.);
        Device {
            name: device_name.into(),
            headset: Headset {
                recommended_eye_width: 1440,
                recommended_eye_height: 1600,
                available_refresh_rates,
                preferred_refresh_rate,
            }
        }
    }

    pub fn get_name(&self) -> &String {
        &self.name
    }

    pub fn get_recommended_eye_width(&self) -> u32 {
        self.headset.recommended_eye_width
    }

    pub fn get_recommended_eye_height(&self) -> u32 {
        self.headset.recommended_eye_height
    }

    pub fn get_available_refresh_rates(&self) -> &Vec<f32> {
        &self.headset.available_refresh_rates
    }

    pub fn get_preferred_refresh_rate(&self) -> f32 {
        self.headset.preferred_refresh_rate
    }
}