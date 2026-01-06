#!/bin/bash
# Android performance profiling script

set -euo pipefail

echo "=== Akroasis Android Performance Profiling ==="
echo ""

# Check device connected
if ! adb devices | grep -q "device$"; then
    echo "Error: No Android device connected"
    exit 1
fi

PACKAGE="app.akroasis"
ACTIVITY="app.akroasis.MainActivity"

echo "📱 Device: $(adb shell getprop ro.product.model)"
echo "🔧 Android: $(adb shell getprop ro.build.version.release)"
echo ""

# Cold start time
echo "=== Cold Start Performance ==="
adb shell am force-stop "$PACKAGE"
sleep 2
START_TIME=$(date +%s%3N)
adb shell am start -W -n "$ACTIVITY" 2>&1 | grep -E "TotalTime|WaitTime"
echo ""

# Memory usage
echo "=== Memory Usage ==="
sleep 5  # Let app stabilize
adb shell dumpsys meminfo "$PACKAGE" | grep -E "TOTAL|Native Heap|Dalvik Heap"
echo ""

# Frame stats (last 120 frames)
echo "=== Frame Statistics ==="
adb shell dumpsys gfxinfo "$PACKAGE" | grep -A 5 "Janky frames"
echo ""

# APK size
echo "=== APK Size ==="
APK_PATH=$(find android/app/build/outputs/apk/debug -name "*.apk" 2>/dev/null | head -1)
if [ -n "$APK_PATH" ]; then
    ls -lh "$APK_PATH" | awk '{print "APK: " $5 " (" $9 ")"}'
fi
echo ""

echo "✅ Profiling complete"
echo "See docs/PERFORMANCE.md for baseline targets"
