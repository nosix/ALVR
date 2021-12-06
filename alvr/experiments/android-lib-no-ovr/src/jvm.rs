use crate::{
    common::ConnectionEvent,
    connection::ConnectionObserver,
    nal::Nal,
};
use alvr_common::prelude::*;
use bytes::Bytes;
use jni::{
    JavaVM, JNIEnv,
    objects::{GlobalRef, JObject, JString, JValue},
};
use serde_json;

const STRING_TYPE: &'static str = "Ljava/lang/String;";

pub struct Preferences<'a> {
    env: JNIEnv<'a>,
    object: JObject<'a>,
}

impl<'a> Preferences<'a> {
    pub fn new(env: JNIEnv<'a>, object: JObject<'a>) -> Preferences<'a> {
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
        self.get_string_field("hostname")
    }

    pub fn set_certificate_pem(&self, value: &str) {
        self.set_string_field("certificate_pem", value)
    }

    pub fn get_certificate_pem(&self) -> String {
        self.get_string_field("certificate_pem")
    }

    pub fn set_key_pem(&self, value: &str) {
        self.set_string_field("key_pem", value)
    }

    pub fn get_key_pem(&self) -> String {
        self.get_string_field("key_pem")
    }

    fn set_string_field(&self, field_name: &str, value: &str) {
        let j_string = self.env.new_string(value).unwrap();
        self.env.set_field(self.object, field_name, STRING_TYPE, j_string.into()).unwrap()
    }

    fn get_string_field(&self, field_name: &str) -> String {
        match self.env.get_field(self.object, field_name, STRING_TYPE).unwrap() {
            JValue::Object(object) => {
                self.env.get_string(JString::from(object)).unwrap().into()
            }
            _ => "".into()
        }
    }
}

pub struct InputBuffer {
    object: GlobalRef,
}

unsafe impl Sync for InputBuffer {}

unsafe impl Send for InputBuffer {}

impl InputBuffer {
    pub fn new(env: JNIEnv, object: JObject) -> StrResult<InputBuffer> {
        Ok(InputBuffer {
            object: trace_err!(env.new_global_ref(object))?,
        })
    }

    pub fn queue_config(&self, env: &JNIEnv, nal: Nal) -> StrResult {
        info!(
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
        info!(
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
    pub fn new(env: &JNIEnv, object: JObject) -> StrResult<JConnectionObserver> {
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