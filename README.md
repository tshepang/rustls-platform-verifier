# platform-tls

[![crates.io version](https://img.shields.io/crates/v/platform-tls.svg)](https://crates.io/crates/platform-tls)
[![crate documentation](https://docs.rs/platform-tls/badge.svg)](https://docs.rs/platform-tls)
![MSRV](https://img.shields.io/badge/rustc-1.56+-blue.svg)
[![crates.io downloads](https://img.shields.io/crates/d/platform-tls.svg)](https://crates.io/crates/platform-tls)
![CI](https://github.com/1Password/platform-tls/workflows/CI/badge.svg)

A Rust library to verify the validity of TLS certificates based on the operating system's certificate facilities.
On operating systems that don't have these, `webpki` and/or `rustls-native-certs` is used instead.

This crate is advantageous over `rustls-native-certs` on its own for a few reasons:
- Improved correctness and security, as the OSes [CA constraints](https://support.apple.com/en-us/HT212865) will be taken into account.
- Better integration with OS certificate stores and enterprise CA deployments.
- Revocation support via verifying validity via OCSP and CRLs.
- Less I/O and memory overhead because all the platform CAs don't need to be loaded and parsed. 

This library supports the following platforms and flows:

| OS             | Certificate Store                             | Verification Method                  | Revocation Support | 
|----------------|-----------------------------------------------|--------------------------------------|--------------------|
| Windows        | Windows platform certificate store            | Windows API certificate verification | Yes                |
| macOS (10.14+) | macOS platform roots and keychain certificate | macOS `Security.framework`           | Yes                |
| iOS            | iOS platform roots and keychain certificates  | iOS `Security.framework`             | Yes                |
| Android        | Android System Trust Store                    | Android Trust Manager                | Sometimes[^1]      |
| Linux          | webpki roots and platform certificate bundles | webpki                               | No[^2]             |
| WASM           | webpki roots                                  | webpki                               | No[^2]             |

[^1]: On Android, revocation checking requires API version >= 24 (e.g. at least Android 7.0, August 2016).
For newer devices that support revocation, Android requires certificates to specify a revocation provider
for network fetch (including optionally stapled OSCP response only applies to chain's end-entity).
This may cause revocation checking to fail for enterprise/internal CAs that don't properly issue an end-entity.

[^2]: <https://docs.rs/rustls/0.20.6/src/rustls/verify.rs.html#341>

## Installation and setup
On most platforms, no setup should be required beyond adding the dependency via `cargo`:
```toml
platform-tls = "0.1"
```

### Android
Some manual setup is required, outside of `cargo`, to use this crate on Android. In order to
use Android's certificate verifier, the crate needs to call into the JVM. A small Kotlin
component must be included in your app's build to support `platform-tls`.

#### Gradle Setup

`platform-tls` bundles the required native components in the crate, but the project must be setup to locate them
automatically and correctly.

Firstly, create an [init script](https://docs.gradle.org/current/userguide/init_scripts.html) in your Android
Gradle project, with a filename of `init.gradle`. This is generally placed in your project's root. In your project's `settings.gradle`, add these lines:

```groovy
apply from: file("./init.gradle");
configurePlatformTlsProject()

// includeFlat is not deprecated, see https://github.com/gradle/gradle/issues/18644#issuecomment-980037131 for more details.
//noinspection GrDeprecatedAPIUsage
includeFlat("platform-tls")

// Note: This path is dependent on where your workspace's target path.
project(":platform-tls").projectDir = file("$rootDir/../target/platform-tls/android/")
```

Next, the `platform-tls` external dependency needs to be setup. Open the `init.gradle` file and add the following, where
`$PATH_TO_ROOT` is replaced with a directory navigation to where your project's `Cargo.toml` is located (ie; `"../"`):

```groovy
ext.configurePlatformTlsProject = {
    def cmdProcessBuilder = new ProcessBuilder(new String[] { "cargo", "check", "-p", "platform-tls" })
    cmdProcessBuilder.directory(File("$PATH_TO_ROOT"))
    cmdProcessBuilder.environment().put("PLATFORM_TLS_GEN_ANDROID_SRC", "1")

    def cmdProcess = cmdProcessBuilder.start()

    cmdProcess.waitFor()
}
```

This script can be tweaked as best suits your project, but the `cargo check` invocation must be included.

Finally, sync your gradle project changes. It should pick up on the `platform-tls` Gradle project and, 
if you look in `./target/platform-tls`, there should be an `android` folder containing the required Kotlin sources. 
After this, everything should be ready to use.Future updates of `platform-tls` shouldn't need anything 
beyond the usual `cargo update` either.

#### Crate initialization

In order for the crate to call into the JVM, it needs handles from Android. These
are provided either the `init_external` or `init_hosted` function. These give `platform-tls`
the resources it needs to make calls into the Android certificate verifier.

As an example, if your Rust Android component which the "native" Android 
part of your app calls at startup has an initialization, like this:
```rust ,ignore
#[export_name = "Java_com_orgname_android_rust_init"]
extern "C" fn java_init(
    env: JNIEnv,
    _class: JClass,
    context: JObject,
) -> jboolean {
    // ... initialize your app's other parts here.
}
```

In the simplest case, you should to insert a call to `platform_tls::android::init_hosted()` here, 
before any networking has a chance to run. This only needs to be called once and
the verifier will be valid for the lifetime of your app's process.

```rust ,ignore
extern "C" fn java_init(
    env: JNIEnv,
    _class: JClass,
    context: JObject,
) -> jboolean {
    // ... initialize your app's other parts here.

    // Then, initialize the certificate verifier for future use.
    platform_tls::android::init_hosted(&env, context);
}
```

In more advanced cases, such as where your code already stores long-lived handles into 
the Android environment, you can alternatively use `init_external`. This function takes
a `&'static` reference to something that implements the `android::Runtime` trait, which the
crate then uses to obtain the access when required to the JVM.

## Credits
Made with ❤️ by the [1Password](https://1password.com/) team. Portions of the Android and Windows implementation
were adapted and referenced from Chromium's verifier implementation as well.

#### License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
</sub>