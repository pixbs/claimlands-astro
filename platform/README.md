# Mobile builds

Both packages run `claimlands-game`; they contain no gameplay or rendering code.
Android loads the Rust `cdylib` through `NativeActivity`. The iOS main function
calls the Rust `staticlib` entrypoint, which lets winit own the UIKit event loop.
No Windows game executable is produced.

Smoke scripts use Python 3 to validate native lifecycle evidence. Run their
device-independent failure-path checks with
`python3 -m unittest discover -s platform/tests -v`.

## Android

Use JDK 17, SDK/build-tools 37, NDK 27.2.12479018, cargo-ndk 4.1.2, and
Gradle 9.3.1. AGP is pinned to 9.1.1. `ANDROID_HOME` points to the SDK;
`JAVA_HOME` and `PATH` must select JDK 17. These are the CI setup/build commands:

```sh
sdkmanager 'platform-tools' 'platforms;android-37' 'build-tools;37.0.0' 'ndk;27.2.12479018'
rustup target add aarch64-linux-android x86_64-linux-android
cargo install cargo-ndk --version 4.1.2 --locked
bash platform/android/build.sh debug
```

The debug APK is `platform/android/app/build/outputs/apk/debug/app-debug.apk` and
includes arm64-v8a devices and x86_64 emulators. Set `CLAIMLANDS_ANDROID_ABIS=x86_64`
for an emulator-only build. Windows contributors can run `gradlew.bat` with
`:app:assembleDebug :app:lint -PrustAbis=arm64-v8a,x86_64` from `platform/android`.
The Gradle tasks invoke Cargo themselves and package the current Rust output.
Release APKs are unsigned; signing and store distribution need separate setup.

On a booted API 37 emulator with a working GLES 3.0/Vulkan implementation:

```sh
ANDROID_SERIAL=emulator-5554 bash platform/android/smoke.sh
```

The test installs the APK, waits for the first rendered frame, drags the globe,
backgrounds and resumes the app, then requires another rendered frame in the
same process. Native errors, crashes, restarts, and missing markers fail it.
Screenshots and logcat are saved under `test-results/android`. Configure a
headless emulator with hardware acceleration and `-gpu swiftshader_indirect`
on Linux CI. Frame rendering must work; a skipped emulator test is not a pass.

## iOS

Use macOS with full Xcode, an installed iOS SDK/simulator runtime, and XcodeGen
2.45.4. Select the Xcode version explicitly in CI. The minimum deployment target
is iOS 15; device builds use arm64, and simulator builds follow the host CPU.

```sh
rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
bash platform/ios/build.sh simulator
bash platform/ios/build.sh device
```

The scripts produce unsigned apps beneath
`platform/ios/DerivedData/{simulator,device}/Build/Products/`. XcodeGen generates
the ignored Xcode project from `project.yml`; commit the specification instead
of editing the generated project. The simulator build also builds its XCTest
runner. No Apple account or provisioning profile is needed for these CI builds;
installing on a physical device requires signing.

Boot an available simulator, then pass its UDID:

```sh
xcrun simctl list devices available
xcrun simctl boot "$SIMULATOR_UDID"
bash platform/ios/smoke.sh "$SIMULATOR_UDID"
```

The script launches with native stderr capture and waits for the first rendered
frame. XCTest attaches to the running game, drags, backgrounds, and reactivates
it; the script requires resume and rendered-frame markers afterward. Errors and
missing evidence fail the run. Logs, screenshots, and the XCTest result bundle
are saved under `test-results/ios`. There are no automatic retry-to-pass paths.

## Build references and verification boundary

- [Android NativeActivity manifest](https://developer.android.com/ndk/samples/sample_na)
- [AGP 8.9 compatibility](https://developer.android.com/build/releases/agp-8-9-0-release-notes)
- [cargo-ndk configuration](https://github.com/bbqsrc/cargo-ndk)
- [winit iOS entrypoint/lifecycle](https://docs.rs/winit/0.30.13/winit/platform/ios/index.html)
- [XcodeGen project specification](https://github.com/yonaskolb/XcodeGen/blob/2.45.4/Docs/ProjectSpec.md)
- [XCTest application state](https://developer.apple.com/documentation/xcuiautomation/xcuiapplication/state-swift.property)

Script syntax, XML/plist parsing, and the Gradle wrapper checksum can be checked
on the development host. Only successful SDK builds and emulator/simulator runs
establish mobile support; source inspection on Windows does not establish it.
