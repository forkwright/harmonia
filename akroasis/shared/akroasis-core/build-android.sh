#!/usr/bin/env bash
# Build Rust library for Android targets

set -e

echo "Building akroasis-core for Android..."

ANDROID_TARGETS=(
    "aarch64-linux-android"
    "armv7-linux-androideabi"
    "x86_64-linux-android"
    "i686-linux-android"
)

for target in "${ANDROID_TARGETS[@]}"; do
    echo "Building for $target..."

    if rustup target list --installed | grep -q "$target"; then
        echo "  Target $target already installed"
    else
        echo "  Installing target $target..."
        rustup target add "$target"
    fi

    cargo build --release --target "$target" --features android

    case "$target" in
        "aarch64-linux-android")
            jni_dir="arm64-v8a"
            ;;
        "armv7-linux-androideabi")
            jni_dir="armeabi-v7a"
            ;;
        "x86_64-linux-android")
            jni_dir="x86_64"
            ;;
        "i686-linux-android")
            jni_dir="x86"
            ;;
    esac

    output_dir="../../android/app/src/main/jniLibs/$jni_dir"
    mkdir -p "$output_dir"

    cp "target/$target/release/libakroasis_core.so" "$output_dir/"
    echo "  Copied libakroasis_core.so to $output_dir"
done

echo ""
echo "Android build complete!"
echo "Libraries copied to android/app/src/main/jniLibs/"
