use crate::{
    catch_err,
    device::{self, Device, DeviceAdapter, Tracking},
};
use alvr_common::prelude::*;
use jni::{
    JavaVM, JNIEnv,
    objects::{GlobalRef, JObject},
};
use once_cell::sync::OnceCell;
use std::{
    ffi::CStr,
    os::raw::c_char,
    slice::from_raw_parts,
};

static PLUGIN: OnceCell<UnityPlugin> = OnceCell::new();

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
    catch_err!({
        let plugin = UnityPlugin {
            vm: env.get_java_vm().unwrap(),
            object: env.new_global_ref(object).unwrap(),
        };
        trace_err!(PLUGIN.set(plugin).map_err(|_| "The PLUGIN is already initialized."))?;
    });
}

#[no_mangle]
extern "system" fn GetInitContextEventFunc() -> *const i32 {
    init_context as *const i32
}

fn init_context(_event_id: i32) {
    catch_err!({
        let plugin = trace_err!(PLUGIN.get()
            .ok_or("The PLUGIN has not been initialized."))?;
        plugin.init_context();
    });
}

#[no_mangle]
extern "system" fn SetDeviceAdapter(
    get_device_settings: extern fn() -> &'static UniDeviceSettings,
    get_tracking: extern fn(i64) -> &'static Tracking,
    on_rendered: extern fn(i64) -> (),
) {
    catch_err!({
        trace_err!(device::set_device_adapter(Box::new(UniDeviceAdapter {
            get_device_csharp_func: get_device_settings,
            get_tracking_func: get_tracking,
            on_rendered_func: on_rendered,
        })))?;
    });
}

struct UniDeviceAdapter<'a> {
    get_device_csharp_func: extern fn() -> &'a UniDeviceSettings,
    get_tracking_func: extern fn(i64) -> &'a Tracking,
    on_rendered_func: extern fn(i64) -> ()
}

impl DeviceAdapter for UniDeviceAdapter<'_> {
    fn get_device(&self) -> StrResult<Device> {
        let device_settings = (self.get_device_csharp_func)();
        Ok(device_settings.into())
    }

    fn get_tracking(&self, frame_index: u64) -> StrResult<Tracking> {
        let tracking = (self.get_tracking_func)(frame_index as i64);
        Ok(*tracking)
    }

    fn on_rendered(&self, frame_index: u64) -> StrResult<()> {
        (self.on_rendered_func)(frame_index as i64);
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
