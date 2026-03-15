# Release process

Automated APK builds and GitHub releases via GitHub Actions.

## Creating a release

1. **Tag the commit**:
   ```bash
   git tag -a v0.1.0 -m "Release v0.1.0"
   git push origin v0.1.0
   ```

2. **GitHub Actions automatically**:
   - Builds debug and release APKs
   - Generates changelog from commits since last tag
   - Creates GitHub release with APK attachments
   - Marks alpha/beta/rc tags as pre-releases

3. **Release published** at `https://github.com/forkwright/harmonia/releases`

## Versioning

Follow semantic versioning: `MAJOR.MINOR.PATCH`

- **MAJOR**: Breaking changes (incompatible API changes)
- **MINOR**: New features (backward compatible)
- **PATCH**: Bug fixes (backward compatible)

Pre-release tags: `v0.1.0-alpha.1`, `v0.1.0-beta.2`, `v0.1.0-rc.1`

## APK signing (optional)

For production releases, configure APK signing via GitHub secrets.

### Generate keystore

```bash
keytool -genkey -v -keystore release.keystore -alias akouo -keyalg RSA -keysize 2048 -validity 10000
```

Answer the prompts and remember:
- Keystore password
- Key alias (use "akouo")
- Key password

### Configure GitHub secrets

Go to `Settings > Secrets and variables > Actions` and add:

| Secret Name | Value |
|-------------|-------|
| `KEYSTORE_BASE64` | Base64-encoded keystore file (see "Encode keystore") |
| `KEYSTORE_PASSWORD` | Keystore password |
| `KEY_ALIAS` | Key alias (e.g., "akouo") |
| `KEY_PASSWORD` | Key password |

**Encode keystore**:
```bash
base64 -w 0 release.keystore > release.keystore.b64
# Copy contents of release.keystore.b64 to KEYSTORE_BASE64 secret
```

### Signing configuration

The release workflow automatically detects if `KEYSTORE_BASE64` secret exists:
- **If present**: Builds signed release APK
- **If absent**: Builds unsigned release APK (still installable for sideloading)

## Release checklist

Before creating a release tag:

- [ ] All tests pass (`./gradlew test`)
- [ ] SonarCloud quality gate passes (web only)
- [ ] CHANGELOG.md updated with release notes
- [ ] Version code incremented in `android/app/build.gradle.kts`
- [ ] Version name matches tag (e.g., `versionName = "0.1.0"` for `v0.1.0`)
- [ ] Phase milestones completed (see ROADMAP.md)
- [ ] Documentation updated

## Version code management

Android requires monotonically increasing version codes. Update in `android/app/build.gradle.kts`:

```kotlin
defaultConfig {
    versionCode = 2  // Increment for each release
    versionName = "0.1.1"  // Match git tag
}
```

## Distribution channels

### GitHub releases (primary)
- Automated via GitHub Actions
- Suitable for self-hosters and power users
- No review process, instant publishing

### F-Droid (future)
- Requires open source compliance (already met)
- Reproducible builds preferred
- See `docs/fdroid-setup.md` when ready

### Google Play (optional)
- Requires developer account ($25 one-time fee)
- Review process (1-3 days)
- Requires signed AAB (not APK)

## Troubleshooting

**Build fails with signing error**:
- Verify all 4 secrets are set correctly
- Check keystore password matches
- Ensure base64 encoding has no line breaks (`-w 0`)

**APK won't install**:
- Enable "Install from Unknown Sources" in Android settings
- For Android 8+: Enable per-app install permission
- Check minimum SDK version (minSdk = 29, Android 10+)

**Release not created**:
- Check GitHub Actions logs
- Ensure tag follows `v*.*.*` pattern
- Verify GITHUB_TOKEN has write permissions

## Manual build (local)

If GitHub Actions unavailable:

```bash
cd android

# Debug APK (no signing needed)
./gradlew assembleDebug
# Output: app/build/outputs/apk/debug/app-debug.apk

# Release APK (requires signing configuration)
./gradlew assembleRelease
# Output: app/build/outputs/apk/release/app-release.apk
```

For local signing, add to `android/local.properties` (not tracked by git):
```properties
KEYSTORE_FILE=/path/to/release.keystore
KEYSTORE_PASSWORD=your_keystore_password
KEY_ALIAS=akouo
KEY_PASSWORD=your_key_password
```
