pub type Percentage = u8;

pub struct Device {
    pub name: String,
    pub recommended_eye_width: u32,
    pub recommended_eye_height: u32,
    pub available_refresh_rates: Vec<f32>,
    pub preferred_refresh_rate: f32,
}

#[repr(C)]
pub struct Tracking {
    /// Inter Pupillary Distance (meter)
    pub ipd: f32,
    pub battery: Percentage,
    pub plugged: u8,
    pub l_eye_fov: Rect,
    pub r_eye_fov: Rect,
    pub head_pose_orientation: Quaternion,
    pub head_pose_position: Vector3,
}

#[repr(C)]
pub struct Rect {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

#[repr(C)]
pub struct Quaternion {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

#[repr(C)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32
}