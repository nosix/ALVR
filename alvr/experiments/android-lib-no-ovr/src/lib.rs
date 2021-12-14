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

use crate::{
    device::Device,
    jvm::{JConnectionObserver, Preferences},
};
use alvr_common::prelude::*;
use alvr_sockets::PrivateIdentity;
use jni::{
    JNIEnv, objects::JObject,
    sys::jboolean,
};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use crate::jvm::InputBuffer;

static DEVICE: Lazy<Device> = Lazy::new(|| Device::new("Android ALVR"));
static MAYBE_IDENTITY: Lazy<Mutex<Option<PrivateIdentity>>> = Lazy::new(|| Mutex::new(None));

/// Execute the $b with the return value $t, call 'show_err' and return Option<$t>.
/// The default of $t is ().
macro_rules! catch_err {
    ($b:block,$t:ty) => {{
        let s = || -> StrResult<$t> {
            Ok($b)
        };
        show_err(s())
    }};
    ($b:block) => {
        catch_err!($b,())
    };
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

        *MAYBE_IDENTITY.lock() = Some(PrivateIdentity {
            hostname: preferences.get_hostname().into(),
            certificate_pem: preferences.get_certificate_pem().into(),
            key_pem: preferences.get_key_pem().into()
        });

        is_changed
    }, bool).unwrap_or(false).into()
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
pub extern "system" fn Java_io_github_alvr_android_lib_NativeApi_onCreate(
    _: JNIEnv,
    _: JObject,
) {
    logging_backend::init_logging();
}

#[no_mangle]
pub extern "system" fn Java_io_github_alvr_android_lib_NativeApi_onStart(
    env: JNIEnv,
    _: JObject,
) {
    catch_err!({
        let vm = trace_err!(env.get_java_vm())?;
        buffer_queue::set_vm(vm);

        let identity = clone_identity(MAYBE_IDENTITY.lock().as_ref()
            .ok_or("Identity has not been initialized. Call initPreferences before onStart.")?);
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

#[no_mangle]
pub extern "system" fn Java_io_github_alvr_android_lib_NativeApi_onInputBufferAvailable(
    env: JNIEnv,
    _: JObject,
    buffer: JObject
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
    frame_index: i64
) {
    latency_controller::INSTANCE.lock().decoder_output(frame_index as u64);
}

#[no_mangle]
pub extern "system" fn Java_io_github_alvr_android_lib_NativeApi_onRendered(
    _: JNIEnv,
    _: JObject,
    frame_index: i64
) {
    connection::on_rendered(frame_index as u64);
}

fn clone_identity(identity: &PrivateIdentity) -> PrivateIdentity {
    PrivateIdentity {
        hostname: identity.hostname.clone(),
        certificate_pem: identity.certificate_pem.clone(),
        key_pem: identity.key_pem.clone()
    }
}