#!/usr/bin/env bash
set -euo pipefail
project_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
workspace_root="$(cd "$project_dir/../.." && pwd)"
apk="${1:-$project_dir/app/build/outputs/apk/debug/app-debug.apk}"
artifacts="${CLAIMLANDS_MOBILE_ARTIFACTS:-$workspace_root/test-results/android}"
package=net.pixbs.claimlands
activity="$package/android.app.NativeActivity"
mkdir -p "$artifacts"
[[ -f "$apk" ]] || { echo "Missing APK: $apk" >&2; exit 1; }
# ANDROID_SERIAL selects a single device when more than one is connected.
adb get-state | grep -qx device
[[ "$(adb shell getprop sys.boot_completed | tr -d '\r')" == 1 ]] || {
  echo 'The emulator/device has not finished booting.' >&2; exit 1;
}
adb install -r "$apk"
adb shell am force-stop "$package"
adb logcat -c
capture_logs() { adb logcat -d -v threadtime > "$artifacts/logcat.txt"; }
trap capture_logs EXIT

assert_alive() {
  local current_pid
  current_pid="$(adb shell pidof "$package" | tr -d '\r')"
  [[ -n "$current_pid" ]] || { echo 'The native game process exited.' >&2; exit 1; }
  if [[ -n "${game_pid:-}" && "$current_pid" != "$game_pid" ]]; then
    echo 'The game restarted instead of resuming its existing process.' >&2; exit 1
  fi
  capture_logs
  if grep -Eq 'ClaimLands error=|FATAL EXCEPTION|Fatal signal [0-9]|ANR in net\.pixbs\.claimlands' "$artifacts/logcat.txt"; then
    cat "$artifacts/logcat.txt" >&2
    exit 1
  fi
}
wait_for_marker() {
  local marker="$1" minimum="$2" count
  for ((attempt=0; attempt<60; attempt++)); do
    assert_alive
    count="$(grep -c "$marker" "$artifacts/logcat.txt" || true)"
    if (( count >= minimum )); then return; fi
    sleep 1
  done
  echo "Timed out waiting for $minimum occurrences of $marker" >&2
  exit 1
}

adb shell am start -W -n "$activity"
game_pid="$(adb shell pidof "$package" | tr -d '\r')"
[[ -n "$game_pid" ]]
wait_for_marker 'ClaimLands ready=true' 1
adb exec-out screencap -p > "$artifacts/initial.png"
# A real touch path and background/foreground cycle exercise the shared handler.
adb shell input swipe 450 800 700 950 400
adb shell input keyevent KEYCODE_HOME
wait_for_marker 'ClaimLands suspended=' 1
adb shell am start -W -n "$activity"
wait_for_marker 'ClaimLands ready=true' 2
wait_for_marker 'ClaimLands resumed=' 2
adb exec-out screencap -p > "$artifacts/resumed.png"
sleep 3
assert_alive
python3 "$workspace_root/platform/verify-lifecycle.py" "$artifacts/logcat.txt"
echo "Android lifecycle smoke passed; evidence: $artifacts"
