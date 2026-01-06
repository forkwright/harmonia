#!/bin/bash
# Create GitHub issues from templates
# Usage: ./CREATE_ISSUES.sh

set -euo pipefail

TEMPLATE_DIR=".github/issue_templates"
REPO="forkwright/akroasis"

echo "Creating GitHub issues from templates in $TEMPLATE_DIR"
echo "Repository: $REPO"
echo ""

# Array of template files in order
TEMPLATES=(
  "issue_54_track_albumid.md"
  "issue_55_phase1_qa.md"
  "issue_56_voice_search.md"
  "issue_57_ab_level_normalization.md"
  "issue_58_battery_profiling.md"
  "issue_59_security_audit.md"
  "issue_61_queue_persistence.md"
  "issue_62_signal_path_format_detection.md"
  "issue_63_test_coverage.md"
  "issue_64_performance_profiling.md"
)

# Check if gh CLI is installed
if ! command -v gh &> /dev/null; then
  echo "Error: gh CLI not found. Install with: sudo dnf install gh"
  exit 1
fi

# Check if authenticated
if ! gh auth status &> /dev/null; then
  echo "Error: Not authenticated with GitHub. Run: gh auth login"
  exit 1
fi

echo "Found ${#TEMPLATES[@]} templates to process"
echo ""

# Process each template
for template in "${TEMPLATES[@]}"; do
  template_path="$TEMPLATE_DIR/$template"
  issue_num="${template#issue_}"
  issue_num="${issue_num%.md}"

  if [ ! -f "$template_path" ]; then
    echo "Warning: Template not found: $template_path"
    continue
  fi

  echo "Creating issue from $template..."

  # Extract title from YAML frontmatter
  title=$(grep "^title:" "$template_path" | sed "s/^title: '\(.*\)'/\1/")

  # Extract labels from YAML frontmatter
  labels=$(grep "^labels:" "$template_path" | sed "s/^labels: '\(.*\)'/\1/")

  # Extract body (everything after --- frontmatter)
  body=$(sed -n '/^---$/,/^---$/!p' "$template_path" | tail -n +2)

  echo "  Title: $title"
  echo "  Labels: $labels"

  # Create issue using gh CLI
  # Note: This will create the issue immediately
  # Use --dry-run flag to preview without creating
  gh issue create \
    --repo "$REPO" \
    --title "$title" \
    --body "$body" \
    --label "$labels"

  echo "  Created successfully!"
  echo ""

  # Small delay to avoid rate limiting
  sleep 2
done

echo "All issues created successfully!"
echo ""
echo "View issues at: https://github.com/$REPO/issues"
