use jni::objects::JObject;
use jni::sys::jstring;
use jni::JNIEnv;

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

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
