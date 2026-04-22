#!/bin/bash
# Vendor foliate-js reader bundle at a pinned SHA with sha256 verification.
# Usage: ./xtask/vendor-reader.sh
#
# This script downloads foliate-js from the upstream GitHub repository,
# verifies the SHA256 checksum, and extracts it into the paroche assets.
# The pinned SHA and expected checksum are hard-coded below to ensure
# reproducible builds and guard against MITM attacks.

set -euo pipefail

# Configuration
UPSTREAM_OWNER="johnfactotum"
UPSTREAM_REPO="foliate-js"
PINNED_SHA="76dcd8f0f7ccabd59199fc5eddbe012d8d463b18"
EXPECTED_CHECKSUM="ae9ac34e06764e8250b3821b3c7c3571d19b221dabb748ff43ffbf3fb63b9a22"
ASSET_DIR="crates/paroche/assets/reader"
TEMP_DIR=$(mktemp -d)
TARBALL_PATH="$TEMP_DIR/foliate-js.tar.gz"
EXTRACT_DIR="$TEMP_DIR/foliate-js-extract"

trap "rm -rf $TEMP_DIR" EXIT

echo "Vendoring foliate-js from github.com/$UPSTREAM_OWNER/$UPSTREAM_REPO @ $PINNED_SHA"
echo "Asset destination: $ASSET_DIR"

# Download the tarball
echo "Downloading foliate-js tarball..."
curl -fsSL \
  "https://github.com/$UPSTREAM_OWNER/$UPSTREAM_REPO/archive/$PINNED_SHA.tar.gz" \
  -o "$TARBALL_PATH"

# Verify checksum
echo "Verifying SHA256 checksum..."
COMPUTED_CHECKSUM=$(sha256sum "$TARBALL_PATH" | awk '{print $1}')

if [ "$COMPUTED_CHECKSUM" != "$EXPECTED_CHECKSUM" ]; then
  echo "ERROR: Checksum mismatch!"
  echo "  Expected: $EXPECTED_CHECKSUM"
  echo "  Got:      $COMPUTED_CHECKSUM"
  exit 1
fi

echo "✓ Checksum verified"

# Extract
echo "Extracting tarball..."
mkdir -p "$EXTRACT_DIR"
tar -xzf "$TARBALL_PATH" -C "$EXTRACT_DIR"

# The extracted dir is foliate-js-<SHA>
EXTRACTED_NAME="$UPSTREAM_REPO-${PINNED_SHA}"
EXTRACTED_PATH="$EXTRACT_DIR/$EXTRACTED_NAME"

if [ ! -d "$EXTRACTED_PATH" ]; then
  echo "ERROR: Expected extracted directory not found: $EXTRACTED_PATH"
  exit 1
fi

# Remove old vendor if it exists
echo "Cleaning old vendor directory..."
rm -rf "$ASSET_DIR/foliate-js-$PINNED_SHA"

# Create asset directory structure
mkdir -p "$ASSET_DIR"

# Copy the bundled reader files
echo "Copying reader bundle..."
cp -r "$EXTRACTED_PATH" "$ASSET_DIR/foliate-js-$PINNED_SHA"

# Verify key files are present
# foliate-js uses ES modules directly (no build step)
REQUIRED_FILES=(
  "LICENSE"
  "view.js"
  "reader.html"
  "reader.js"
)

for file in "${REQUIRED_FILES[@]}"; do
  if [ ! -f "$ASSET_DIR/foliate-js-$PINNED_SHA/$file" ]; then
    echo "ERROR: Required file not found: $file"
    exit 1
  fi
done

echo "✓ Copying LICENSE..."
cp "$ASSET_DIR/foliate-js-$PINNED_SHA/LICENSE" "$ASSET_DIR/LICENSE.foliate-js"

echo "✓ Successfully vendored foliate-js"
echo ""
echo "Main entry point: /static/reader/foliate-js-$PINNED_SHA/view.js"
echo "Reference reader: /static/reader/foliate-js-$PINNED_SHA/reader.html"
