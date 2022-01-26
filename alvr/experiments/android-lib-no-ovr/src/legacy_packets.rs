use crate::device::{Quaternion, Vector3};
use bincode;
use bytes::Bytes;
use serde::{Serialize, Deserialize};

#[derive(Debug, PartialEq)]
pub enum AlvrPacketType {
    TrackingInfo,
    TimeSync,
    VideoFrame,
    PacketErrorReport,
    HapticsFeedback,
    Unknown,
}

impl From<u32> for AlvrPacketType {
    fn from(n: u32) -> AlvrPacketType {
        match n {
            6 => AlvrPacketType::TrackingInfo,
            7 => AlvrPacketType::TimeSync,
            9 => AlvrPacketType::VideoFrame,
            12 => AlvrPacketType::PacketErrorReport,
            13 => AlvrPacketType::HapticsFeedback,
            _ => AlvrPacketType::Unknown
        }
    }
}

impl From<AlvrPacketType> for u32 {
    fn from(t: AlvrPacketType) -> u32 {
        match t {
            AlvrPacketType::TrackingInfo => 6,
            AlvrPacketType::TimeSync => 7,
            AlvrPacketType::VideoFrame => 9,
            AlvrPacketType::PacketErrorReport => 12,
            AlvrPacketType::HapticsFeedback => 13,
            AlvrPacketType::Unknown => u32::MAX
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum AlvrLostFrameType {
    Video,
    Unknown,
}

impl From<AlvrLostFrameType> for u32 {
    fn from(value: AlvrLostFrameType) -> u32 {
        match value {
            AlvrLostFrameType::Video => 0,
            AlvrLostFrameType::Unknown => u32::MAX
        }
    }
}

const ALVR_HAND_BONE_LENGTH: usize = 19;

enum AlvrHandBone {
    WristRoot, // root frame of the hand, where the wrist is located
    ForearmStub, // frame for user's forearm
    Thumb0, // thumb trapezium bone
    Thumb1, // thumb metacarpal bone
    Thumb2, // thumb proximal phalange bone
    Thumb3, // thumb distal phalange bone
    Index1, // index proximal phalange bone
    Index2, // index intermediate phalange bone
    Index3, // index distal phalange bone
    Middle1, // middle proximal phalange bone
    Middle2, // middle intermediate phalange bone
    Middle3, // middle distal phalange bone
    Ring1, // ring proximal phalange bone
    Ring2, // ring intermediate phalange bone
    Ring3, // ring distal phalange bone
    Pinky0, // pinky metacarpal bone
    Pinky1, // pinky proximal phalange bone
    Pinky2, // pinky intermediate phalange bone
    Pinky3, // pinky distal phalange bone
}

const ALVR_FINGER_PINCH_LENGTH: usize = 4;

enum AlvrFingerPinch {
    Index,
    Middle,
    Ring,
    Pinky,
}

pub const CONTROLLER_FLAG_ENABLE: u32 = 1 << 0;
pub const CONTROLLER_FLAG_LEFT_HAND: u32 = 1 << 1;
pub const CONTROLLER_FLAG_GEAR_VR: u32 = 1 << 2;
pub const CONTROLLER_FLAG_OCULUS_GO: u32 = 1 << 3;
pub const CONTROLLER_FLAG_OCULUS_QUEST: u32 = 1 << 4;
pub const CONTROLLER_FLAG_OCULUS_HAND: u32 = 1 << 5;

pub const INPUT_FLAG_SYSTEM_CLICK: u64 = 1 << 0;
pub const INPUT_FLAG_APPLICATION_MENU_CLICK: u64 = 1 << 1;
pub const INPUT_FLAG_GRIP_CLICK: u64 = 1 << 2;
pub const INPUT_FLAG_GRIP_VALUE: u64 = 1 << 3;
pub const INPUT_FLAG_GRIP_TOUCH: u64 = 1 << 4;
pub const INPUT_FLAG_DPAD_LEFT_CLICK: u64 = 1 << 5;
pub const INPUT_FLAG_DPAD_UP_CLICK: u64 = 1 << 6;
pub const INPUT_FLAG_DPAD_RIGHT_CLICK: u64 = 1 << 7;
pub const INPUT_FLAG_DPAD_DOWN_CLICK: u64 = 1 << 8;
pub const INPUT_FLAG_A_CLICK: u64 = 1 << 9;
pub const INPUT_FLAG_A_TOUCH: u64 = 1 << 10;
pub const INPUT_FLAG_B_CLICK: u64 = 1 << 11;
pub const INPUT_FLAG_B_TOUCH: u64 = 1 << 12;
pub const INPUT_FLAG_X_CLICK: u64 = 1 << 13;
pub const INPUT_FLAG_X_TOUCH: u64 = 1 << 14;
pub const INPUT_FLAG_Y_CLICK: u64 = 1 << 15;
pub const INPUT_FLAG_Y_TOUCH: u64 = 1 << 16;
pub const INPUT_FLAG_TRIGGER_LEFT_VALUE: u64 = 1 << 17;
pub const INPUT_FLAG_TRIGGER_RIGHT_VALUE: u64 = 1 << 18;
pub const INPUT_FLAG_SHOULDER_LEFT_CLICK: u64 = 1 << 19;
pub const INPUT_FLAG_SHOULDER_RIGHT_CLICK: u64 = 1 << 20;
pub const INPUT_FLAG_JOYSTICK_LEFT_CLICK: u64 = 1 << 21;
pub const INPUT_FLAG_JOYSTICK_LEFT_X: u64 = 1 << 22;
pub const INPUT_FLAG_JOYSTICK_LEFT_Y: u64 = 1 << 23;
pub const INPUT_FLAG_JOYSTICK_RIGHT_CLICK: u64 = 1 << 24;
pub const INPUT_FLAG_JOYSTICK_RIGHT_X: u64 = 1 << 25;
pub const INPUT_FLAG_JOYSTICK_RIGHT_Y: u64 = 1 << 26;
pub const INPUT_FLAG_JOYSTICK_CLICK: u64 = 1 << 27;
pub const INPUT_FLAG_JOYSTICK_X: u64 = 1 << 28;
pub const INPUT_FLAG_JOYSTICK_Y: u64 = 1 << 29;
pub const INPUT_FLAG_JOYSTICK_TOUCH: u64 = 1 << 30;
pub const INPUT_FLAG_BACK_CLICK: u64 = 1 << 31;
pub const INPUT_FLAG_GUIDE_CLICK: u64 = 1 << 32;
pub const INPUT_FLAG_START_CLICK: u64 = 1 << 33;
pub const INPUT_FLAG_TRIGGER_CLICK: u64 = 1 << 34;
pub const INPUT_FLAG_TRIGGER_VALUE: u64 = 1 << 35;
pub const INPUT_FLAG_TRIGGER_TOUCH: u64 = 1 << 36;
pub const INPUT_FLAG_TRACKPAD_X: u64 = 1 << 37;
pub const INPUT_FLAG_TRACKPAD_Y: u64 = 1 << 38;
pub const INPUT_FLAG_TRACKPAD_CLICK: u64 = 1 << 39;
pub const INPUT_FLAG_TRACKPAD_TOUCH: u64 = 1 << 40;
pub const INPUT_FLAG_THUMB_REST_TOUCH: u64 = 1 << 41;

#[repr(C, packed)]
#[derive(Debug, Serialize, Deserialize)]
pub struct TrackingInfo {
    pub packet_type: u32, // AlvrPacketType::TrackingInfo
    pub flags: u32,
    pub client_time: u64,
    pub frame_index: u64,
    pub predicted_display_time: f64,
    pub head_pose_orientation: TrackingQuad,
    pub head_pose_position: TrackingVector3,
    pub head_pose_angular_velocity: TrackingVector3,
    pub head_pose_linear_velocity: TrackingVector3,
    pub head_pose_angular_acceleration: TrackingVector3,
    pub head_pose_linear_acceleration: TrackingVector3,
    pub other_tracking_source_position: TrackingVector3,
    pub other_tracking_source_orientation: TrackingQuad,
    pub eye_fov: [EyeFov; 2], // FOV of left and right eyes.
    pub ipd: f32,
    pub battery: u64,
    pub plugged: u8,
    pub mounted: u8,
    pub controller: [Controller; 2]
}

impl From<Bytes> for TrackingInfo {
    fn from(buffer: Bytes) -> TrackingInfo { deserialize(buffer) }
}

impl From<TrackingInfo> for Vec<u8> {
    fn from(packet: TrackingInfo) -> Vec<u8> { serialize(packet) }
}

impl Default for TrackingInfo {
    fn default() -> TrackingInfo {
        TrackingInfo {
            packet_type: AlvrPacketType::TrackingInfo.into(),
            flags: 0,
            client_time: 0,
            frame_index: 0,
            predicted_display_time: 0.,
            head_pose_orientation: Default::default(),
            head_pose_position: Default::default(),
            head_pose_angular_velocity: Default::default(),
            head_pose_linear_velocity: Default::default(),
            head_pose_angular_acceleration: Default::default(),
            head_pose_linear_acceleration: Default::default(),
            other_tracking_source_position: Default::default(),
            other_tracking_source_orientation: Default::default(),
            eye_fov: [EyeFov::left_default(), EyeFov::right_default()],
            ipd: 0.,
            battery: 0,
            plugged: 0,
            mounted: 0,
            controller: Default::default()
        }
    }
}

#[repr(C, packed)]
#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
pub struct Controller {
    pub flags: u32,
    pub buttons: u64,
    pub trackpad_position_x: f32,
    pub trackpad_position_y: f32,
    pub trigger_value: f32,
    pub grip_value: f32,
    pub battery_percent_remaining: u8,
    pub recenter_count: u8,
    // Tracking info of controller. (float * 19 = 76 bytes)
    pub orientation: TrackingQuad,
    pub position: TrackingVector3,
    pub angular_velocity: TrackingVector3,
    pub linear_velocity: TrackingVector3,
    pub angular_acceleration: TrackingVector3,
    pub linear_acceleration: TrackingVector3,
    // Tracking info of hand. A3
    pub bone_rotations: [TrackingQuad; ALVR_HAND_BONE_LENGTH],
    pub bone_positions_base: [TrackingVector3; ALVR_HAND_BONE_LENGTH],
    pub bone_root_orientation: TrackingQuad,
    pub bone_root_position: TrackingVector3,
    pub input_state_status: u32,
    pub finger_pinch_strengths: [f32; ALVR_FINGER_PINCH_LENGTH],
    pub hand_finger_confidence: u32
}

impl Default for Controller {
    fn default() -> Controller {
        Controller {
            flags: 0,
            buttons: 0,
            trackpad_position_x: 0.,
            trackpad_position_y: 0.,
            trigger_value: 0.,
            grip_value: 0.,
            battery_percent_remaining: 0,
            recenter_count: 0,
            orientation: Default::default(),
            position: Default::default(),
            angular_velocity: Default::default(),
            linear_velocity: Default::default(),
            angular_acceleration: Default::default(),
            linear_acceleration: Default::default(),
            bone_rotations: Default::default(),
            bone_positions_base: Default::default(),
            bone_root_orientation: Default::default(),
            bone_root_position: Default::default(),
            input_state_status: 0,
            finger_pinch_strengths: Default::default(),
            hand_finger_confidence: 0
        }
    }
}

#[repr(C, packed)]
#[derive(Debug, Serialize, Deserialize, Default, Copy, Clone)]
pub struct TrackingQuad {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32
}

impl From<Quaternion> for TrackingQuad {
    fn from(value: Quaternion) -> Self {
        Self {
            x: value.x,
            y: value.y,
            z: value.z,
            w: value.w,
        }
    }
}

#[repr(C, packed)]
#[derive(Debug, Serialize, Deserialize, Default, Copy, Clone)]
pub struct TrackingVector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32
}

impl From<Vector3> for TrackingVector3 {
    fn from(value: Vector3) -> Self {
        Self {
            x: value.x,
            y: value.y,
            z: value.z,
        }
    }
}

#[repr(C, packed)]
#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
pub struct EyeFov {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32
}

impl EyeFov {
    fn left_default() -> EyeFov {
        Default::default()
    }

    fn right_default() -> EyeFov {
        let left = EyeFov::left_default();
        EyeFov {
            left: left.right,
            right: left.left,
            ..Default::default()
        }
    }
}

impl Default for EyeFov {
    // Represent FOV for each eye in degree. Default is left eye for Quest 2
    fn default() -> EyeFov {
        EyeFov {
            left: 49.,
            right: 45.,
            top: 50.,
            bottom: 48.
        }
    }
}

#[repr(C, packed)]
#[derive(Debug, Serialize, Deserialize)]
pub struct TimeSync {
    pub packet_type: u32, // AlvrPacketType::TimeSync

    // Client >----(mode 0)----> Server
    // Client <----(mode 1)----< Server
    // Client >----(mode 2)----> Server
    // Client <----(mode 3)----< Server
    pub mode: u32, // 0,1,2,3

    pub sequence: u64,
    pub server_time: u64,
    pub client_time: u64,

    // Following value are filled by client only when mode=0.
    pub packets_lost_total: u64,
    pub packets_lost_in_second: u64,

    pub average_total_latency: u32,
    pub average_send_latency: u32,
    pub average_transport_latency: u32,
    pub average_decode_latency: u64,
    pub idle_time: u32,

    pub fec_failure: u32,
    pub fec_failure_in_second: u64,
    pub fec_failure_total: u64,

    pub fps: f32,

    // Following value are filled by server only when mode=1.
    pub server_total_latency: u32,

    // Following value are filled by server only when mode=3.
    pub tracking_recv_frame_index: u64,
}

impl From<Bytes> for TimeSync {
    fn from(buffer: Bytes) -> TimeSync { deserialize(buffer) }
}

impl From<TimeSync> for Vec<u8> {
    fn from(packet: TimeSync) -> Vec<u8> { serialize(packet) }
}

impl Default for TimeSync {
    fn default() -> Self {
        Self {
            packet_type: AlvrPacketType::TimeSync.into(),
            mode: 0,
            sequence: 0,
            server_time: 0,
            client_time: 0,
            packets_lost_total: 0,
            packets_lost_in_second: 0,
            average_total_latency: 0,
            average_send_latency: 0,
            average_transport_latency: 0,
            average_decode_latency: 0,
            idle_time: 0,
            fec_failure: 0,
            fec_failure_in_second: 0,
            fec_failure_total: 0,
            fps: 0.,
            server_total_latency: 0,
            tracking_recv_frame_index: 0,
        }
    }
}

#[derive(Debug)]
pub struct VideoFrame {
    pub header: VideoFrameHeader,
    pub payload: Bytes
}

#[repr(C, packed)]
#[derive(Debug, Serialize, Deserialize)]
pub struct VideoFrameHeader {
    pub packet_type: u32, // AlvrPacketType::VideoFrame
    pub packet_counter: u32,
    pub tracking_frame_index: u64,
    // FEC decoder needs some value for identify video frame number to detect new frame.
    // trackingFrameIndex becomes sometimes same value as previous video frame
    // (in case of low tracking rate).
    pub video_frame_index: u64,
    pub sent_time: u64,
    pub frame_byte_size: u32,
    pub fec_index: u32,
    pub fec_percentage: u16,
    // Followed by frame buffer as the body
}

impl From<Bytes> for VideoFrameHeader {
    fn from(buffer: Bytes) -> VideoFrameHeader { deserialize(buffer) }
}

impl Default for VideoFrameHeader {
    fn default() -> VideoFrameHeader {
        VideoFrameHeader {
            packet_type: AlvrPacketType::VideoFrame.into(),
            packet_counter: 0,
            tracking_frame_index: 0,
            video_frame_index: 0,
            sent_time: 0,
            frame_byte_size: 0,
            fec_index: 0,
            fec_percentage: 0,
        }
    }
}

#[repr(C, packed)]
#[derive(Debug, Serialize, Deserialize)]
pub struct PacketErrorReport {
    pub packet_type: u32, // AlvrPacketType::PacketErrorReport
    pub lost_frame_type: u32,
    pub from_packet_counter: u32,
    pub to_packet_counter: u32
}

impl Default for PacketErrorReport {
    fn default() -> Self {
        Self {
            packet_type: AlvrPacketType::PacketErrorReport.into(),
            lost_frame_type: AlvrLostFrameType::Video.into(),
            from_packet_counter: 0,
            to_packet_counter: 0
        }
    }
}

impl From<Bytes> for PacketErrorReport {
    fn from(buffer: Bytes) -> PacketErrorReport { deserialize(buffer) }
}

impl From<PacketErrorReport> for Vec<u8> {
    fn from(packet: PacketErrorReport) -> Vec<u8> { serialize(packet) }
}

#[repr(C, packed)]
#[derive(Debug, Serialize, Deserialize)]
pub struct HapticsFeedback {
    pub packet_type: u32, // AlvrPacketType::HapticsFeedback
    pub start_time: u64, // Elapsed time from now when start haptics. In microseconds.
    pub amplitude: f32,
    pub duration: f32,
    pub frequency: f32,
    pub hand: u8, // 0:Right, 1:Left
}

impl From<Bytes> for HapticsFeedback {
    fn from(buffer: Bytes) -> HapticsFeedback { deserialize(buffer) }
}

// TODO check length
fn deserialize<T>(buffer: Bytes) -> T {
    unsafe { std::ptr::read(buffer.as_ptr() as *const _) }
}

fn serialize<T>(packet: T) -> Vec<u8> where T : Serialize {
    bincode::serialize(&packet).unwrap()
}