# Versioning and breaking change policy

## Version scheme

Semantic versioning with pre-1.0 interpretation:

| Version | Meaning |
|---------|---------|
| `0.MINOR.PATCH` | Pre-stable. Breaking changes allowed in MINOR bumps with documented migration |
| `1.0.0` | Stable public API. Breaking changes require MAJOR bump |

## Per-component versioning

Harmonia is a monorepo. Each component maintains its own version independently:

| Component | Version Source | Current |
|-----------|---------------|---------|
| Mouseion | `legacy/mouseion/*.csproj` (PropertyGroup > Version) | Pre-stable |
| Akroasis Web | `legacy/web/package.json` | Pre-stable |
| Akroasis Android | `legacy/android/app/build.gradle` (versionName) | Pre-stable |

Components are versioned independently; a breaking change in Mouseion does not require bumping Akroasis.

## What constitutes a breaking change

### Breaking (requires MINOR bump)
- Changing API endpoint signatures (request/response shape)
- Removing or renaming a config key
- Changing database schema without automatic migration
- Removing or renaming a CLI command

### Non-breaking (PATCH bump)
- Adding new API endpoints
- Adding new config keys with defaults
- Adding new database columns with defaults
- Bug fixes
- Performance improvements
- Documentation changes

## Migration path

Every breaking change includes:
1. **Migration guide** in the release notes
2. **Automated migration** where possible (schema migrations, config transformers)
3. **Deprecation period** of at least one minor release for removals

## Release process

1. Bump version in the relevant component's version source
2. Update `CHANGELOG.md` with categorized changes
3. Tag: `git tag <component>-v0.MINOR.PATCH`
4. GitHub Release with migration notes if breaking

## Changelog format

```markdown
## [mouseion-v0.2.0] — 2026-MM-DD

### Breaking
- Changed audiobook chapter detection endpoint response shape

### Added
- New metadata provider for MusicBrainz

### Fixed
- Chapter duration rounding error on files >2h
```
