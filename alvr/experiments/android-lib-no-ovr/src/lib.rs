mod audio;
mod buffer_queue;
mod common;
mod connection;
mod device;
mod fec;
mod jvm;
mod latency_controller;
mod legacy_packets;
mod legacy_stream;
mod logging_backend;
mod nal;
mod unity;
mod util;

use crate::jvm::{
    InputBuffer,
    JConnectionObserver, JDeviceAdapter,
    Preferences,
};
use alvr_common::prelude::*;
use alvr_sockets::PrivateIdentity;
use jni::{
    JavaVM, JNIEnv, JNIVersion,
    objects::JObject,
    sys::{jboolean, jint},
};
use std::ffi::c_void;

#[no_mangle]
pub unsafe extern "system" fn JNI_OnLoad(vm: JavaVM, _reserved: *const c_void) -> jint {
    logging_backend::init_logging();
    buffer_queue::set_vm(vm);
    JNIVersion::V6.into()
}

#[no_mangle]
pub extern "system" fn Java_io_github_alvr_android_lib_NativeApi_initPreferences(
    env: JNIEnv,
    _: JObject,
    preferences: JObject,
) -> jboolean {
    catch_err!({
        let preferences = Preferences::new(env, preferences);
        let mut is_changed = false;

        if preferences.is_empty() {
            let identity = trace_err!(alvr_sockets::create_identity(None))?;
            preferences.set_hostname(&identity.hostname);
            preferences.set_certificate_pem(&identity.certificate_pem);
            preferences.set_key_pem(&identity.key_pem);
            is_changed = true;
        };

        let identity = PrivateIdentity {
            hostname: preferences.get_hostname().into(),
            certificate_pem: preferences.get_certificate_pem().into(),
            key_pem: preferences.get_key_pem().into()
        };
        trace_err!(device::set_identity(identity))?;

        is_changed
    }, bool).unwrap_or(false).into()
}

#[no_mangle]
pub extern "system" fn Java_io_github_alvr_android_lib_NativeApi_setDeviceAdapter(
    env: JNIEnv,
    _: JObject,
    request: JObject,
) {
    catch_err!({
        let wrapper = trace_err!(JDeviceAdapter::new(&env, request))?;
        trace_err!(device::set_device_adapter(Box::new(wrapper)))?;
    });
}

#[no_mangle]
pub extern "system" fn Java_io_github_alvr_android_lib_NativeApi_setConnectionObserver(
    env: JNIEnv,
    _: JObject,
    observer: JObject,
) {
    catch_err!({
        let observer = JConnectionObserver::new(&env, observer)?;
        connection::set_observer(Box::new(observer));
    });
}

#[no_mangle]
pub extern "system" fn Java_io_github_alvr_android_lib_NativeApi_onStart(
    _: JNIEnv,
    _: JObject,
) {
    catch_err!({
        let device = trace_err!(device::get_device())?;
        let identity = trace_err!(device::get_identity())?;
        trace_err!(connection::connect(device, identity))?;
    });
}

#[no_mangle]
pub extern "system" fn Java_io_github_alvr_android_lib_NativeApi_onStop(
    _: JNIEnv,
    _: JObject,
) {
    connection::disconnect();
}

#[no_mangle]
pub extern "system" fn Java_io_github_alvr_android_lib_NativeApi_onInputBufferAvailable(
    env: JNIEnv,
    _: JObject,
    buffer: JObject,
) {
    catch_err!({
        let input_buffer = InputBuffer::new(env, buffer)?;
        buffer_queue::push_input_buffer(input_buffer)?;
    });
}

#[no_mangle]
pub extern "system" fn Java_io_github_alvr_android_lib_NativeApi_onOutputBufferAvailable(
    _: JNIEnv,
    _: JObject,
    frame_index: i64,
) {
    latency_controller::decoder_output(frame_index as u64);
}

#[no_mangle]
pub extern "system" fn Java_io_github_alvr_android_lib_NativeApi_onRendered(
    _: JNIEnv,
    _: JObject,
    frame_index: i64,
) {
    catch_err!({
        latency_controller::rendered1(frame_index as u64);
        trace_err!(device::on_rendered(frame_index as u64))?;
        connection::on_rendered(frame_index as u64);
    });
}
