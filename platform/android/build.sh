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
[[ "$java_version" == *'version "21.'* ]] || {
  echo 'Set JAVA_HOME and PATH to JDK 21.' >&2; exit 1;
}
if bash "$project_dir/gradlew" --project-dir "$project_dir" --no-daemon \
  ":app:$task" :app:lint "$@"; then
  exit 0
else
  result=$?
  report="$project_dir/app/build/reports/lint-results.txt"
  if [[ -f "$report" ]]; then cat "$report" >&2; fi
  exit "$result"
fi
