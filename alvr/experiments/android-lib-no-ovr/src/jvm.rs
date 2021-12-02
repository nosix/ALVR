use jni::objects::{JObject, JValue, JString};
use jni::JNIEnv;

const STRING_TYPE: &'static str = "Ljava/lang/String;";

pub struct Preferences<'a> {
    env: JNIEnv<'a>,
    object: JObject<'a>
}

impl<'a> Preferences<'a> {
    pub fn new(env: JNIEnv<'a>, object: JObject<'a>) -> Preferences<'a> {
        Preferences {
            env,
            object
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

pub struct InputBuffer<'a> {
    env: JNIEnv<'a>,
    object: JObject<'a>
}

impl<'a> InputBuffer<'a> {
    pub fn new(env: JNIEnv<'a>, object: JObject<'a>) -> InputBuffer<'a> {
        InputBuffer {
            env,
            object
        }
    }
}