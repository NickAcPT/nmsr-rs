use std::fmt::{Debug, Display};

use jni::JNIEnv;

macro_rules! get_string_or_throw {
    ($env: expr, $expr: expr, $default_return: expr) => {
        unwrap_or_throw_java_exception!(
            $env,
            $env.get_string($expr)
                .map(|s| s.to_str().unwrap_or("").to_owned()),
            $default_return
        )
    };
}

macro_rules! unwrap_or_throw_java_exception {
    ($env: expr, $error_expr:expr, $default: expr) => {
        match $error_expr {
            Ok(v) => v,
            Err(e) => {
                crate::error_handling::ErrorAsJavaException::throw_java_exception(&e, &mut $env);
                return $default;
            }
        }
    };
}

pub(crate) use get_string_or_throw;
pub(crate) use unwrap_or_throw_java_exception;

pub(crate) trait ErrorAsJavaException {
    fn throw_java_exception(&self, env: &mut JNIEnv);
}

impl<E: Display + Debug> ErrorAsJavaException for E {
    fn throw_java_exception(&self, env: &mut JNIEnv) {
        env.throw_new(
            "io/github/nickacpt/jnmsr/natives/JNMSRException",
            format!("{:?}", &self),
        )
        .expect("Unable to throw error as Java exception");
    }
}
