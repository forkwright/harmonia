# Release Process

Mouseion follows semantic versioning (MAJOR.MINOR.PATCH) with sparse, meaningful releases.

## Release Cadence

- **Major releases** (v2.0.0, v3.0.0): Breaking changes, significant architecture shifts
- **Minor releases** (v1.1.0, v1.2.0): New features, functionality additions
- **Patch releases** (v1.0.1, v1.0.2): Bug fixes, security updates

**Philosophy**: Release when there's value to ship, not on a schedule. Quality over frequency.

## Creating a Release

### 1. Prepare the Release

```bash
# Update CHANGELOG.md
# Add new version section at top:
## [1.2.0] - 2026-01-15

### Added
- Feature X for media type Y
- API endpoint Z

### Fixed
- Bug in component A
- Issue with service B

# Commit changelog
git add CHANGELOG.md
git commit -m "docs: prepare v1.2.0 release"
git push origin develop
```

### 2. Create and Push Version Tag

```bash
# Tag the release (triggers GitHub Actions)
git tag -a v1.2.0 -m "Release v1.2.0"
git push origin v1.2.0
```

### 3. Automated Process

GitHub Actions automatically:
- Extracts changelog for this version
- Builds linux-x64 and win-x64 binaries
- Creates release archives
- Publishes GitHub Release with binaries

### 4. Post-Release

- Verify release on GitHub
- Update documentation if needed
- Announce in appropriate channels

## Version Bumping Guidelines

**Patch** (1.0.x):
- Bug fixes only
- No new features
- No breaking changes
- Safe to auto-update

**Minor** (1.x.0):
- New features
- Backwards compatible
- API additions (no removals)
- Database migrations (backwards compatible)

**Major** (x.0.0):
- Breaking API changes
- Removed features
- Incompatible database schema changes
- Requires user action to upgrade

## What NOT to Release

- Development snapshots
- Every PR merge
- Experimental features
- Incomplete functionality

Releases are **promises** to users. Ship complete, tested features.

## Pre-releases (Optional)

For testing before stable release:

```bash
git tag -a v1.2.0-rc1 -m "Release candidate 1 for v1.2.0"
git push origin v1.2.0-rc1
```

Pre-releases (tags with `-alpha`, `-beta`, `-rc`) are marked as such on GitHub.

## Release Artifacts

Each release includes:
- **linux-x64**: Linux binary (tar.gz)
- **win-x64**: Windows binary (zip)
- **Source code**: Automatic GitHub archive
- **Changelog**: Extracted from CHANGELOG.md

## Troubleshooting

**Release workflow failed?**
- Check GitHub Actions logs
- Verify CHANGELOG.md has entry for version
- Ensure tag matches pattern `v[0-9]+.[0-9]+.[0-9]+`

**Need to delete a release?**
```bash
# Delete tag locally and remotely
git tag -d v1.2.0
git push origin :refs/tags/v1.2.0

# Delete release on GitHub UI
# Then re-create if needed
```
