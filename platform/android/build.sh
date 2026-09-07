#!/usr/bin/env bash
set -euo pipefail
project_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
variant="${1:-debug}"
if [[ $# -gt 0 ]]; then shift; fi
case "$variant" in
  debug) task=assembleDebug ;;
  release) task=assembleRelease ;;
  *) echo "Usage: bash platform/android/build.sh [debug|release]" >&2; exit 2 ;;
esac
command -v cargo >/dev/null
cargo ndk --version | grep -Eq 'cargo-ndk 4\.1\.2$' || {
  echo 'Install cargo-ndk 4.1.2 with cargo install cargo-ndk --version 4.1.2 --locked' >&2
  exit 1
}
java_version="$(java -version 2>&1)"
[[ "$java_version" == *'version "17.'* ]] || {
  echo 'Set JAVA_HOME and PATH to JDK 17.' >&2; exit 1;
}
bash "$project_dir/gradlew" --project-dir "$project_dir" --no-daemon \
  ":app:$task" :app:lint "$@" "-PrustAbis=${CLAIMLANDS_ANDROID_ABIS:-arm64-v8a,x86_64}"
