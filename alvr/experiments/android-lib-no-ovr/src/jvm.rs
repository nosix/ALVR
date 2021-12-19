use crate::{
    common::ConnectionEvent,
    connection::ConnectionObserver,
    device::Device,
    nal::Nal,
    store::DeviceDataProducer,
};
use alvr_common::prelude::*;
use bytes::Bytes;
use jni::{
    JavaVM, JNIEnv,
    objects::{GlobalRef, JObject, JString, JValue},
};
use serde_json;

const INT_TYPE: &'static str = "I";
const FLOAT_TYPE: &'static str = "F";
const FLOAT_ARRAY_TYPE: &'static str = "[F";
const STRING_TYPE: &'static str = "Ljava/lang/String;";

fn get_int_field(env: &JNIEnv, object: JObject, field_name: &str) -> i32 {
    match env.get_field(object, field_name, INT_TYPE).unwrap() {
        JValue::Int(value) => value,
        _ => 0
    }
}

fn get_float_field(env: &JNIEnv, object: JObject, field_name: &str) -> f32 {
    match env.get_field(object, field_name, FLOAT_TYPE).unwrap() {
        JValue::Float(value) => value,
        _ => 0.0
    }
}

fn get_string_field(env: &JNIEnv, object: JObject, field_name: &str) -> String {
    match env.get_field(object, field_name, STRING_TYPE).unwrap() {
        JValue::Object(object) => {
            env.get_string(JString::from(object)).unwrap().into()
        }
        _ => "".into()
    }
}

fn get_float_array_field(env: &JNIEnv, object: JObject, field_name: &str) -> Vec<f32> {
    match env.get_field(object, field_name, FLOAT_ARRAY_TYPE).unwrap() {
        JValue::Object(object) => {
            let length = env.get_array_length(*object).unwrap();
            let mut buffer = vec![0.0f32; length as usize];
            env.get_float_array_region(
                *object,
                0,
                buffer.as_mut_slice(),
            ).unwrap();
            buffer
        }
        _ => Vec::new()
    }
}

pub struct Preferences<'a> {
    env: JNIEnv<'a>,
    object: JObject<'a>,
}

impl<'a> Preferences<'a> {
    pub fn new(env: JNIEnv<'a>, object: JObject<'a>) -> Self {
        Preferences {
            env,
            object,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.get_hostname().is_empty() ||
            self.get_certificate_pem().is_empty() ||
            self.get_key_pem().is_empty()
    }

    pub fn set_hostname(&self, value: &str) {
        self.set_string_field("hostname", value)
    }

    pub fn get_hostname(&self) -> String {
        get_string_field(&self.env, self.object, "hostname")
    }

    pub fn set_certificate_pem(&self, value: &str) {
        self.set_string_field("certificate_pem", value)
    }

    pub fn get_certificate_pem(&self) -> String {
        get_string_field(&self.env, self.object, "certificate_pem")
    }

    pub fn set_key_pem(&self, value: &str) {
        self.set_string_field("key_pem", value)
    }

    pub fn get_key_pem(&self) -> String {
        get_string_field(&self.env, self.object, "key_pem")
    }

    fn set_string_field(&self, field_name: &str, value: &str) {
        let j_string = self.env.new_string(value).unwrap();
        self.env.set_field(self.object, field_name, STRING_TYPE, j_string.into()).unwrap()
    }
}

pub struct JDeviceSettings<'a> {
    env: JNIEnv<'a>,
    object: JObject<'a>,
}

impl<'a> JDeviceSettings<'a> {
    pub fn new(env: JNIEnv<'a>, object: JObject<'a>) -> Self {
        JDeviceSettings {
            env,
            object,
        }
    }

    pub fn get_name(&self) -> String {
        get_string_field(&self.env, self.object, "name")
    }

    pub fn get_recommended_eye_width(&self) -> u32 {
        get_int_field(&self.env, self.object, "recommendedEyeWidth") as u32
    }

    pub fn get_recommended_eye_height(&self) -> u32 {
        get_int_field(&self.env, self.object, "recommendedEyeHeight") as u32
    }

    pub fn get_available_refresh_rates(&self) -> Vec<f32> {
        get_float_array_field(&self.env, self.object, "availableRefreshRates")
    }

    pub fn get_preferred_refresh_rate(&self) -> f32 {
        get_float_field(&self.env, self.object, "preferredRefreshRate")
    }
}

impl From<JDeviceSettings<'_>> for Device {
    fn from(settings: JDeviceSettings) -> Self {
        Device {
            name: settings.get_name(),
            recommended_eye_width: settings.get_recommended_eye_width(),
            recommended_eye_height: settings.get_recommended_eye_height(),
            available_refresh_rates: settings.get_available_refresh_rates(),
            preferred_refresh_rate: settings.get_preferred_refresh_rate(),
        }
    }
}

pub struct InputBuffer {
    object: GlobalRef,
}

unsafe impl Sync for InputBuffer {}

unsafe impl Send for InputBuffer {}

impl InputBuffer {
    pub fn new(env: JNIEnv, object: JObject) -> StrResult<Self> {
        Ok(InputBuffer {
            object: trace_err!(env.new_global_ref(object))?,
        })
    }

    pub fn queue_config(&self, env: &JNIEnv, nal: Nal) -> StrResult {
        debug!(
            "queue_config {:?} frame_len={} frame_index={}",
            nal.nal_type, nal.frame_buffer.len(), nal.frame_index
        );
        self.copy_buffer(env, &nal.frame_buffer);
        trace_err!(env.call_method(
            &self.object, "queueConfig", "()V", &[]
        ))?;
        Ok(())
    }

    pub fn queue(&self, env: &JNIEnv, nal: Nal) -> StrResult {
        debug!(
            "queue {:?} frame_len={} frame_index={}",
            nal.nal_type, nal.frame_buffer.len(), nal.frame_index
        );
        self.copy_buffer(env, &nal.frame_buffer);
        trace_err!(env.call_method(
            &self.object, "queue", "(J)V", &[(nal.frame_index as i64).into()]
        ))?;
        Ok(())
    }

    fn copy_buffer(&self, env: &JNIEnv, frame_buffer: &Bytes) {
        let ret_value = trace_err!(env.call_method(
            &self.object, "getBuffer", "()Ljava/nio/ByteBuffer;", &[]
        )).unwrap();
        if let JValue::Object(byte_buffer) = ret_value {
            let buffer = trace_err!(env.get_direct_buffer_address(byte_buffer.into())).unwrap();
            buffer[..frame_buffer.len()].copy_from_slice(&frame_buffer);
            trace_err!(env.call_method(
                byte_buffer, "position", "(I)Ljava/nio/Buffer;",
                &[(frame_buffer.len() as i32).into()]
            )).unwrap();
        } else {
            panic!("Can't get the byte buffer.");
        }
    }
}

pub struct JConnectionObserver {
    vm: JavaVM,
    object: GlobalRef,
}

impl JConnectionObserver {
    pub fn new(env: &JNIEnv, object: JObject) -> StrResult<Self> {
        Ok(JConnectionObserver {
            vm: trace_err!(env.get_java_vm())?,
            object: trace_err!(env.new_global_ref(object))?,
        })
    }
}

impl ConnectionObserver for JConnectionObserver {
    fn on_event_occurred(&self, event: ConnectionEvent) -> StrResult {
        let env = trace_err!(self.vm.attach_current_thread_permanently())?;
        let json_data = trace_err!(serde_json::to_string(&event))?;
        trace_err!(env.call_method(
            &self.object, "onEventOccurred", "(Ljava/lang/String;)V", &[
                trace_err!(env.new_string(json_data))?.into()
            ]
        ))?;
        Ok(())
    }
}

pub struct JDeviceDataProducer {
    vm: JavaVM,
    object: GlobalRef,
}

impl JDeviceDataProducer {
    pub fn new(env: &JNIEnv, object: JObject) -> StrResult<Self> {
        Ok(JDeviceDataProducer {
            vm: trace_err!(env.get_java_vm())?,
            object: trace_err!(env.new_global_ref(object))?,
        })
    }
}

impl DeviceDataProducer for JDeviceDataProducer {
    fn get_device(&self) -> StrResult<Device> {
        let env = trace_err!(self.vm.attach_current_thread_permanently())?;
        let ret = trace_err!(env.call_method(
            &self.object,
            "getDeviceSettings",
            "()Lio/github/alvr/android/lib/DeviceSettings;",
            &[]
        ))?;
        let device_settings = JDeviceSettings::new(env, ret.l().unwrap());
        Ok(Device {
            name: device_settings.get_name(),
            recommended_eye_width: device_settings.get_recommended_eye_width(),
            recommended_eye_height: device_settings.get_recommended_eye_height(),
            available_refresh_rates: device_settings.get_available_refresh_rates(),
            preferred_refresh_rate: device_settings.get_preferred_refresh_rate()
        })
    }
}