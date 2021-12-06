/// A data format common to Rust and Kotlin (Java).
/// It is sent from Rust to Kotlin (Java) in JSON format.

use alvr_session::CodecType;
use serde::Serialize;
use std::net::IpAddr;

#[derive(Debug, Serialize, PartialEq)]
#[serde(tag = "type")]
pub enum AlvrCodec {
    H264,
    H265,
    Unknown,
}

impl From<u32> for AlvrCodec {
    fn from(n: u32) -> AlvrCodec {
        match n {
            0 => AlvrCodec::H264,
            1 => AlvrCodec::H265,
            _ => AlvrCodec::Unknown
        }
    }
}

impl From<CodecType> for AlvrCodec {
    fn from(t: CodecType) -> AlvrCodec {
        match t {
            CodecType::H264 => AlvrCodec::H264,
            CodecType::HEVC => AlvrCodec::H265,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum ConnectionEvent {
    Initial,
    ServerFound { ipaddr: IpAddr },
    Connected { settings: ConnectionSettings },
    StreamStart,
    ServerRestart,
    Error { error: ConnectionError },
}

#[derive(Debug, Serialize)]
pub struct ConnectionSettings {
    pub fps: f32,
    pub codec: AlvrCodec,
    pub realtime: bool,
    pub dark_mode: bool,
    pub dashboard_url: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(tag = "type")]
pub enum ConnectionError {
    NetworkUnreachable,
    ClientUntrusted,
    IncompatibleVersions,
    TimeoutSetUpStream,
    ServerDisconnected { cause: String },
    SystemError { cause: String },
}

impl From<String> for ConnectionError {
    fn from(cause: String) -> ConnectionError {
        ConnectionError::SystemError { cause }
    }
}