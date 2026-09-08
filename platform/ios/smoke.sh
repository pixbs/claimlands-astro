#!/usr/bin/env bash
set -euo pipefail
[[ "$(uname -s)" == Darwin ]] || { echo 'iOS smoke tests require macOS.' >&2; exit 1; }
project_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
workspace_root="$(cd "$project_dir/../.." && pwd)"
device="${1:?Pass the booted simulator UDID as the first argument}"
artifacts="${CLAIMLANDS_MOBILE_ARTIFACTS:-$workspace_root/test-results/ios}"
products="$project_dir/DerivedData/simulator/Build/Products"
app="$products/Release-iphonesimulator/ClaimLands.app"
bundle=net.pixbs.claimlands
[[ -d "$app" ]] || { echo 'Run platform/ios/build.sh simulator first.' >&2; exit 1; }
mkdir -p "$artifacts"
artifacts="$(cd "$artifacts" && pwd)"
# Each run gets a new result bundle; no stale evidence can satisfy this test.
run_dir="$(mktemp -d "$artifacts/run.XXXXXX")"
xcrun simctl bootstatus "$device" -b
xcrun simctl install "$device" "$app"
touch "$run_dir/stdout.log" "$run_dir/stderr.log"
launch_output="$(xcrun simctl launch --terminate-running-process \
  --stdout="$run_dir/stdout.log" --stderr="$run_dir/stderr.log" "$device" "$bundle")"
printf '%s\n' "$launch_output" > "$run_dir/launch.txt"

assert_no_error() {
  if grep -Eq 'ClaimLands error=|panicked at|fatal runtime error' "$run_dir/stderr.log"; then
    cat "$run_dir/stderr.log" >&2; exit 1
  fi
}
wait_for_marker() {
  local marker="$1" minimum="$2" count
  for ((attempt=0; attempt<60; attempt++)); do
    assert_no_error
    count="$(grep -c "$marker" "$run_dir/stderr.log" || true)"
    if (( count >= minimum )); then return; fi
    sleep 1
  done
  echo "Timed out waiting for $minimum occurrences of $marker" >&2
  cat "$run_dir/stderr.log" >&2
  exit 1
}
wait_for_marker 'ClaimLands ready=true' 1
xcrun simctl io "$device" screenshot "$run_dir/initial.png"

# Tests attach to the already running process, perform a drag, press Home, then
# activate the same app. Native stderr stays attached across the entire cycle.
xctestruns=("$products"/*.xctestrun)
[[ ${#xctestruns[@]} -eq 1 && -f "${xctestruns[0]}" ]] || {
  echo 'Expected one build-for-testing .xctestrun product.' >&2; exit 1;
}
xcodebuild test-without-building -xctestrun "${xctestruns[0]}" \
  -destination "platform=iOS Simulator,id=$device" \
  -parallel-testing-enabled NO \
  -only-testing:ClaimLandsUITests/LifecycleTests/testBackgroundResume \
  -resultBundlePath "$run_dir/Lifecycle.xcresult"
wait_for_marker 'ClaimLands suspended=' 1
wait_for_marker 'ClaimLands resumed=' 2
wait_for_marker 'ClaimLands ready=true' 2
assert_no_error
python3 "$workspace_root/platform/verify-lifecycle.py" "$run_dir/stderr.log"
xcrun simctl io "$device" screenshot "$run_dir/resumed.png"
echo "iOS lifecycle smoke passed; evidence: $run_dir"
