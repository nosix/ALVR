mod audio;
mod connection;
mod device;
mod legacy_packets;
mod logging_backend;
mod util;

use crate::device::Device;
use alvr_common::prelude::*;
use jni::{JNIEnv, objects::JObject, sys::jstring};
use once_cell::sync::Lazy;

static DEVICE: Lazy<Device> = Lazy::new(|| Device::new("Android ALVR"));

macro_rules! catch_err {
    ( $b:block ) => {
        let s = || -> StrResult {
            $b
            Ok(())
        };
        show_err(s());
    }
}

#[no_mangle]
pub extern "system" fn Java_io_github_alvr_android_lib_NativeApi_stringFromJni(
    env: JNIEnv,
    _this: JObject,
) -> jstring {
    let hello = "Hello from Rust";

    env.new_string(hello)
        .expect("Couldn't create Java string!")
        .into_inner()
}

#[no_mangle]
pub extern "system" fn Java_io_github_alvr_android_lib_NativeApi_onCreate(
    _: JNIEnv,
    _: JObject,
) {
    logging_backend::init_logging();
}

#[no_mangle]
pub extern "system" fn Java_io_github_alvr_android_lib_NativeApi_onStart(
    _: JNIEnv,
    _: JObject,
) {
    catch_err!({
        let identity = trace_err!(alvr_sockets::create_identity(None))?;
        trace_err!(connection::connect(&DEVICE, identity))?;
    });
}

#[no_mangle]
pub extern "system" fn Java_io_github_alvr_android_lib_NativeApi_onStop(
    _: JNIEnv,
    _: JObject,
) {
    connection::disconnect();
}