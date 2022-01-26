use crate::{
    device::{
        Controller,
        Quaternion,
        Rect,
        Vector3,
    },
    legacy_packets::TimeSync,
};
use alvr_common::{
    glam::{Quat, Vec3},
    Fov,
};
use alvr_sockets::TimeSyncPacket;

pub const DEFAULT_IPD: f32 = 0.068606;

const DEFAULT_EYE_FOV_INNER: f32 = 45.;
const DEFAULT_EYE_FOV_OUTER: f32 = 49.;
const DEFAULT_EYE_FOV_TOP: f32 = 50.;
const DEFAULT_EYE_FOV_BOTTOM: f32 = 48.;

pub fn default_left_eye_fov() -> Fov {
    Fov {
        left: DEFAULT_EYE_FOV_OUTER,
        right: DEFAULT_EYE_FOV_INNER,
        top: DEFAULT_EYE_FOV_TOP,
        bottom: DEFAULT_EYE_FOV_BOTTOM,
    }
}

pub fn default_right_eye_fov() -> Fov {
    Fov {
        left: DEFAULT_EYE_FOV_INNER,
        right: DEFAULT_EYE_FOV_OUTER,
        top: DEFAULT_EYE_FOV_TOP,
        bottom: DEFAULT_EYE_FOV_BOTTOM,
    }
}

impl From<Rect> for Fov {
    fn from(value: Rect) -> Self {
        Self {
            left: value.left,
            right: value.right,
            top: value.top,
            bottom: value.bottom,
        }
    }
}

impl From<Quaternion> for Quat {
    fn from(value: Quaternion) -> Self {
        Quat::from_xyzw(value.x, value.y, value.z, value.w)
    }
}

impl From<Vector3> for Vec3 {
    fn from(value: Vector3) -> Self {
        Vec3::new(value.x, value.y, value.z)
    }
}

impl Controller {
    pub fn orientation(&self) -> Quat {
        self.orientation.into()
    }

    pub fn position(&self) -> Vec3 {
        self.position.into()
    }

    pub fn linear_velocity(&self) -> Option<Vec3> {
        None
    }

    pub fn angular_velocity(&self) -> Option<Vec3> {
        None
    }
}

impl From<TimeSync> for TimeSyncPacket {
    fn from(value: TimeSync) -> Self {
        Self {
            mode: value.mode,
            server_time: value.server_time,
            client_time: value.client_time,
            packets_lost_total: value.packets_lost_total,
            packets_lost_in_second: value.packets_lost_in_second,
            average_send_latency: value.average_send_latency,
            average_transport_latency: value.average_transport_latency,
            average_decode_latency: value.average_decode_latency,
            idle_time: value.idle_time,
            fec_failure: value.fec_failure,
            fec_failure_in_second: value.fec_failure_in_second,
            fec_failure_total: value.fec_failure_total,
            fps: value.fps,
            server_total_latency: value.server_total_latency,
            tracking_recv_frame_index: value.tracking_recv_frame_index,
        }
    }
}
