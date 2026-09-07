#!/usr/bin/env bash
set -euo pipefail
[[ "$(uname -s)" == Darwin ]] || {
  echo 'iOS builds require macOS with the full Xcode installation selected.' >&2; exit 1;
}
project_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
workspace_root="$(cd "$project_dir/../.." && pwd)"
platform="${1:-simulator}"
case "$platform" in
  simulator)
    sdk=iphonesimulator
    destination='generic/platform=iOS Simulator'
    case "$(uname -m)" in
      arm64) rust_target=aarch64-apple-ios-sim; architecture=arm64 ;;
      x86_64) rust_target=x86_64-apple-ios; architecture=x86_64 ;;
      *) echo 'Unsupported macOS host architecture.' >&2; exit 1 ;;
    esac
    action=build-for-testing
    ;;
  device)
    sdk=iphoneos
    destination='generic/platform=iOS'
    rust_target=aarch64-apple-ios
    architecture=arm64
    action=build
    ;;
  *) echo 'Usage: bash platform/ios/build.sh [simulator|device]' >&2; exit 2 ;;
esac
command -v cargo >/dev/null
command -v xcodebuild >/dev/null
xcodegen --version | grep -Eq 'Version: 2\.45\.4$' || {
  echo 'Install XcodeGen 2.45.4 before building.' >&2; exit 1;
}
export IPHONEOS_DEPLOYMENT_TARGET=15.0
export CARGO_TARGET_DIR="$workspace_root/target"
cd "$workspace_root"
cargo build --locked --package claimlands-game --lib --release --target "$rust_target"
[[ -s "$CARGO_TARGET_DIR/$rust_target/release/libclaimlands_game.a" ]]
xcodegen generate --spec "$project_dir/project.yml"
xcodebuild -project "$project_dir/ClaimLands.xcodeproj" -scheme ClaimLands \
  -configuration Release -sdk "$sdk" -destination "$destination" \
  -derivedDataPath "$project_dir/DerivedData/$platform" \
  "ARCHS=$architecture" ONLY_ACTIVE_ARCH=YES CODE_SIGNING_ALLOWED=NO CODE_SIGNING_REQUIRED=NO \
  "CLAIMLANDS_RUST_TARGET=$rust_target" CLAIMLANDS_RUST_PROFILE=release "$action"
[[ -d "$project_dir/DerivedData/$platform/Build/Products/Release-$sdk/ClaimLands.app" ]]
echo "Unsigned $platform app: $project_dir/DerivedData/$platform/Build/Products/Release-$sdk/ClaimLands.app"
