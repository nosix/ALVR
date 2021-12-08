use jni::{JNIEnv, objects::JObject};
use once_cell::sync::Lazy;
use jni::{
    JavaVM,
    objects::GlobalRef,
};
use parking_lot::Mutex;

static PLUGIN: Lazy<Mutex<Option<UnityPlugin>>> = Lazy::new(|| Mutex::new(None));

struct UnityPlugin {
    vm: JavaVM,
    object: GlobalRef,
}

impl UnityPlugin {
    fn init_context(&self) {
        let env = self.vm.attach_current_thread().unwrap();
        env.call_method(&self.object , "initContext", "()V", &[]).unwrap();
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