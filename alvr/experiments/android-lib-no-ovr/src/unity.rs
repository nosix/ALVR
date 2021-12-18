use crate::{
    catch_err,
    device::Device,
    store::{self, DeviceDataProducer},
};
use alvr_common::prelude::*;
use jni::{
    JavaVM, JNIEnv,
    objects::{GlobalRef, JObject},
};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::{
    ffi::CStr,
    os::raw::c_char,
    slice::from_raw_parts,
};

// TODO change to OnceCell
static PLUGIN: Lazy<Mutex<Option<UnityPlugin>>> = Lazy::new(|| Mutex::new(None));

struct UnityPlugin {
    vm: JavaVM,
    object: GlobalRef,
}

impl UnityPlugin {
    fn init_context(&self) {
        let env = self.vm.attach_current_thread().unwrap();
        env.call_method(&self.object, "initContext", "()V", &[]).unwrap();
    }
}

#[no_mangle]
pub extern "system" fn Java_io_github_alvr_android_lib_UnityPlugin_attach(
    env: JNIEnv,
    object: JObject,
) {
    *PLUGIN.lock() = Some(UnityPlugin {
        vm: env.get_java_vm().unwrap(),
        object: env.new_global_ref(object).unwrap(),
    })
}

#[no_mangle]
extern "system" fn GetInitContextEventFunc() -> *const i32 {
    init_context as *const i32
}

fn init_context(_event_id: i32) {
    if let Some(plugin) = PLUGIN.lock().as_ref() {
        plugin.init_context();
    }
}

struct UniDeviceDataProducer {
    csharp_func: extern fn(i8),
}

impl DeviceDataProducer for UniDeviceDataProducer {
    fn request(&self, data_kind: i8) -> StrResult {
        (self.csharp_func)(data_kind);
        Ok(())
    }
}

#[repr(C)]
pub struct UniDeviceSettings {
    pub name: *const c_char,
    pub recommended_eye_width: i32,
    pub recommended_eye_height: i32,
    pub available_refresh_rates: *const f32,
    pub available_refresh_rates_len: i32,
    pub preferred_refresh_rate: f32,
}

impl From<&UniDeviceSettings> for Device {
    fn from(settings: &UniDeviceSettings) -> Self {
        let name = unsafe {
            CStr::from_ptr(settings.name)
                .to_string_lossy()
                .into_owned()
        };
        let available_refresh_rates = unsafe {
            from_raw_parts(
                settings.available_refresh_rates,
                settings.available_refresh_rates_len as usize)
                .to_vec()
        };

        Device {
            name,
            recommended_eye_width: settings.recommended_eye_width as u32,
            recommended_eye_height: settings.recommended_eye_height as u32,
            available_refresh_rates,
            preferred_refresh_rate: settings.preferred_refresh_rate,
        }
    }
}

#[no_mangle]
extern "system" fn SetDeviceDataProducer(producer: extern fn(i8)) {
    catch_err!({
        trace_err!(store::set_data_producer(Box::new(UniDeviceDataProducer {
            csharp_func: producer
        })))?;
    });
}

#[no_mangle]
extern "system" fn SetDeviceSettings(settings: &UniDeviceSettings) {
    catch_err!({
        trace_err!(store::set_device(settings.into()))?;
    });
}
