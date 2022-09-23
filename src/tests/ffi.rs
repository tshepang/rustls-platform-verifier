//! Thin wrappers ontop the existing test suites that allow them to be ran
//! in the context of a platform-native environment as required by the verifier implementation.

#[cfg(target_os = "android")]
pub use android::*;
#[cfg(target_os = "android")]
mod android {
    //! Tests which run inside the context of a Android device, typically an emulator.
    //!
    //! Note: These tests run inside the same application context, so they share the same mock test
    //! store. This will remain non-problematic as long as roots are different enough (for the use case) and
    //! real roots are never removed from the store.
    //!
    //! It is intentional that the tests run sequentially, as dropping a `Verifier` will reset its mock
    //! root store.
    use crate::tests;
    use jni::{
        objects::{JClass, JObject, JString},
        sys::jstring,
        JNIEnv,
    };
    use std::sync::Once;

    static ANDROID_INIT: Once = Once::new();

    /// A marker that the Kotlin test runner looks for to determine
    /// if a set of integration tests passed or not.
    const SUCCESS_MARKER: &str = "success";

    fn run_android_test<'a>(
        env: &'a JNIEnv,
        cx: JObject,
        suite_name: &'static str,
        test_cases: &'static [fn()],
    ) -> JString<'a> {
        // These can't fail, and even if they did, Android will crash the process like we want.
        ANDROID_INIT.call_once(|| {
            android_logger::init_once(
                android_logger::Config::default().with_min_level(log::Level::Trace),
            );
            crate::android::init_hosted(env, cx).unwrap();
            std::panic::set_hook(Box::new(|info| {
                let msg = if let Some(msg) = info.payload().downcast_ref::<&'static str>() {
                    msg
                } else if let Some(msg) = info.payload().downcast_ref::<String>() {
                    msg.as_str()
                } else {
                    "no panic info available"
                };

                log::error!("test panic: {}", msg)
            }))
        });

        let mut failed = false;
        for test in test_cases.iter() {
            // Use a dedicated thread so panics don't cause crashes.
            if std::thread::spawn(test).join().is_err() {
                failed = true;
                log::error!("{}: test failed", suite_name);
            } else {
                log::info!("{}: test passed", suite_name);
            }

            log::info!(
                "-----------------------------------------------------------------------------"
            )
        }

        env.new_string(if failed {
            "test failed!"
        } else {
            SUCCESS_MARKER
        })
        .unwrap()
    }

    #[export_name = "Java_com_onepassword_platform_1tls_CertificateVerifierTests_mockTests"]
    pub extern "C" fn platform_tls_mock_test_suite(
        env: JNIEnv,
        _class: JClass,
        cx: JObject,
    ) -> jstring {
        log::info!("running mock test suite...");

        run_android_test(
            &env,
            cx,
            "mock tests",
            tests::verification_mock::ALL_TEST_CASES,
        )
        .into_inner()
    }

    #[export_name = "Java_com_onepassword_platform_1tls_CertificateVerifierTests_verifyMockRootUsage"]
    pub extern "C" fn platform_tls_verify_mock_root_usage(
        env: JNIEnv,
        _class: JClass,
        cx: JObject,
    ) -> jstring {
        log::info!("verifying mock roots are not used by default...");

        run_android_test(
            &env,
            cx,
            "mock root verification",
            &[tests::verification_mock::verification_without_mock_root],
        )
        .into_inner()
    }

    #[export_name = "Java_com_onepassword_platform_1tls_CertificateVerifierTests_realWorldTests"]
    pub extern "C" fn platform_tls_real_world_test_suite(
        env: JNIEnv,
        _class: JClass,
        cx: JObject,
    ) -> jstring {
        log::info!("running real world suite...");

        run_android_test(
            &env,
            cx,
            "real world",
            tests::verification_real_world::ALL_TEST_CASES,
        )
        .into_inner()
    }
}

#[cfg(not(target_os = "android"))]
mod dummy {
    //! A module to prevent dead-code warnings all over
    //! the `tests` module due to the weird combination of
    //! feature flags and `--all-features`. These test case
    //! lists are only used via the FFI.

    use crate::tests;

    #[allow(dead_code)]
    fn dummy() {
        #[cfg(any(
            windows,
            target_os = "android",
            target_os = "macos",
            target_os = "linux"
        ))]
        let _ = tests::verification_mock::ALL_TEST_CASES;
        let _ = tests::verification_real_world::ALL_TEST_CASES;
    }
}
