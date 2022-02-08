use alvr_common::prelude::*;
use alvr_sockets::PrivateIdentity;
use once_cell::sync::OnceCell;

pub type Percentage = u8;

pub trait DeviceAdapter: Sync + Send {
    fn get_device(&self) -> StrResult<Device>;
    fn get_tracking(&self, frame_index: u64) -> StrResult<Tracking>;
    fn on_rendered(&self, frame_index: u64) -> StrResult<()>;
}

pub struct Device {
    pub name: String,
    pub recommended_eye_width: u32,
    pub recommended_eye_height: u32,
    pub available_refresh_rates: Vec<f32>,
    pub preferred_refresh_rate: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Tracking {
    /// Inter Pupillary Distance (meter)
    pub ipd: f32,
    pub battery: Percentage,
    pub plugged: u8,
    pub mounted: u8,
    pub l_eye_fov: Rect,
    pub r_eye_fov: Rect,
    pub head_pose_orientation: Quaternion,
    pub head_pose_position: Vector3,
    pub l_ctrl: Controller,
    pub r_ctrl: Controller,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Controller
{
    pub enabled: u8,
    pub buttons: u64,
    pub trackpad_position_x: f32,
    pub trackpad_position_y: f32,
    pub trigger_value: f32,
    pub grip_value: f32,
    pub orientation: Quaternion,
    pub position: Vector3,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Rect {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Quaternion {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32
}

static DEVICE: OnceCell<Device> = OnceCell::new();
static IDENTITY: OnceCell<PrivateIdentity> = OnceCell::new();
static DEVICE_ADAPTER: OnceCell<Box<dyn DeviceAdapter>> = OnceCell::new();

pub fn set_identity(identity: PrivateIdentity) -> StrResult {
    IDENTITY.set(identity)
        .map_err(|_| "The IDENTITY is already set and will not change.".into())
}

pub fn get_identity() -> StrResult<&'static PrivateIdentity> {
    IDENTITY.get()
        .ok_or("The IDENTITY has not been initialized.".into())
}

pub fn set_device_adapter(adapter: Box<dyn DeviceAdapter>) -> StrResult {
    DEVICE_ADAPTER.set(adapter)
        .map_err(|_| "The DEVICE_ADAPTER is already set and will not change.".into())
}

pub fn get_device() -> StrResult<&'static Device> {
    Ok(DEVICE.get_or_init(|| {
        let adapter = DEVICE_ADAPTER.get()
            .expect("The DEVICE_ADAPTER has not been initialized.");
        adapter.get_device()
            .expect("The DEVICE_ADAPTER can't produce a Device instance.")
    }))
}

pub fn get_tracking(frame_index: u64) -> StrResult<Tracking> {
    let adapter = trace_err!(DEVICE_ADAPTER.get()
        .ok_or("The DEVICE_ADAPTER has not been initialized."))?;
    let tracking = trace_err!(adapter.get_tracking(frame_index))?;
    Ok(tracking)
}

pub fn on_rendered(frame_index: u64) -> StrResult<()> {
    let adapter = trace_err!(DEVICE_ADAPTER.get()
        .ok_or("The DEVICE_ADAPTER has not been initialized."))?;
    trace_err!(adapter.on_rendered(frame_index))?;
    Ok(())
}