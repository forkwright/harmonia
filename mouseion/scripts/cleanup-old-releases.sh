#!/usr/bin/env bash
# Cleanup old GitHub releases - keep only meaningful semantic versions
#
# Usage: ./scripts/cleanup-old-releases.sh [--dry-run]
#
# This script removes:
# - Development/test releases
# - Duplicate releases
# - Non-semantic version releases
#
# Keeps:
# - Latest release
# - All semantic version releases (v1.0.0, v1.1.0, etc.)
# - Pre-releases marked as such (v1.0.0-rc1)

set -euo pipefail

DRY_RUN=false
if [[ "${1:-}" == "--dry-run" ]]; then
    DRY_RUN=true
    echo "DRY RUN MODE - No changes will be made"
    echo
fi

echo "Fetching all releases..."
RELEASES=$(gh release list --limit 1000 --json tagName,name,isPrerelease,createdAt)

if [ -z "$RELEASES" ] || [ "$RELEASES" == "[]" ]; then
    echo "No releases found"
    exit 0
fi

echo "Found $(echo "$RELEASES" | jq length) releases"
echo

# Extract releases to delete (non-semantic versions, test releases, etc.)
TO_DELETE=$(echo "$RELEASES" | jq -r '.[] | select(
    (.tagName | test("^v[0-9]+\\.[0-9]+\\.[0-9]+(-[a-z0-9.]+)?$") | not)
) | .tagName')

if [ -z "$TO_DELETE" ]; then
    echo "No releases to clean up - all releases follow semantic versioning"
    exit 0
fi

echo "Releases to delete (non-semantic versions):"
echo "$TO_DELETE"
echo
echo "Count: $(echo "$TO_DELETE" | wc -l)"
echo

if [ "$DRY_RUN" = true ]; then
    echo "DRY RUN - Would delete the above releases"
    exit 0
fi

read -p "Delete these releases? (y/N) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Cancelled"
    exit 1
fi

echo "Deleting releases..."
while IFS= read -r tag; do
    echo "Deleting release: $tag"
    gh release delete "$tag" --yes
    # Also delete the tag
    git push origin ":refs/tags/$tag" 2>/dev/null || true
    echo "  ✓ Deleted"
done <<< "$TO_DELETE"

echo
echo "Cleanup complete!"
echo "Remaining releases:"
gh release list --limit 20
