# Contributing to Akroasis

Unified media player for audiobooks, ebooks, and music with bit-perfect audio playback.

## Project Status

✅ **Phases 0, 1, 3, 6, 7 Complete** - Android client feature-rich, Web MVP in progress

**Android**: 21 major features across 4 phases (365+ tests, 40-50% coverage)
**Web**: MVP development starting (React 19 + Vite)

See [ROADMAP.md](ROADMAP.md) for detailed status and [CHANGELOG.md](CHANGELOG.md) for recent achievements.

## Before Contributing

1. **Review documentation**: [README.md](README.md), [ROADMAP.md](ROADMAP.md)
2. **Check existing issues** - Avoid duplicates
3. **Understand architecture** - Client-only (no server UI). Connects to [Mouseion](https://github.com/forkwright/mouseion) API

## Development Setup

### Prerequisites
- **Android**: Android Studio, Kotlin plugin, Android SDK 14+
- **Web**: Node.js 20+, npm
- **Backend**: [Mouseion](https://github.com/forkwright/mouseion) instance (API server)

### Getting Started
```bash
# Clone repository
git clone https://github.com/forkwright/akroasis.git
cd akroasis

# Android development
cd android
./gradlew build

# Web development
cd web
npm install
npm run dev
```

### Scrobbling Credentials (Android)

Last.fm and ListenBrainz scrobbling require API credentials. Add to `android/local.properties` (not tracked by git):

```properties
lastfm.api.key=your_key_here
lastfm.api.secret=your_secret_here
```

**Get credentials:**
- Last.fm: https://www.last.fm/api/account/create
- Build system injects via BuildConfig
- Falls back to environment variables: `LASTFM_API_KEY`, `LASTFM_API_SECRET`

## Git Workflow

**Branch Structure:**
- `main`: Stable releases only (tagged)
- `develop`: Default branch, integration target for all PRs
- `feature/*`: New features (branch from develop)
- `fix/*`: Bug fixes (branch from develop)

**Process:**
1. Fork and clone the repository
2. Create feature branch from `develop`: `git checkout -b feature/your-feature`
3. Make changes and commit with conventional format: `feat(scope): description`
4. Push to your fork: `git push origin feature/your-feature`
5. Create PR targeting `develop` branch
6. User reviews and merges (squash merge to keep history clean)

## How to Contribute

### Reporting Bugs
Use the bug report template. Include:
- Device/OS/browser version
- Steps to reproduce
- Expected vs. actual behavior
- Relevant logs or screenshots

### Suggesting Features
Use the feature request template. Explain:
- Use case and problem being solved
- Proposed solution
- Alternatives considered

### Code Contributions

See **Git Workflow** section above for branching and PR process.

**Development Standards:**
- Conventional commits: `feat(scope): description`
- No placeholder code
- Test changes before committing
- Match existing code patterns
- Ensure CI passes before requesting review

## Code Standards

- **Kotlin (Android)**: Follow Kotlin style guide, use ktlint
- **TypeScript/JavaScript (Web)**: ESLint + Prettier
- **Documentation**: Concise, technical, accurate
- **Tests**: Required for new features

## Mouseion Integration

Akroasis is a client for the [Mouseion](https://github.com/forkwright/mouseion) backend. When working on API-related features:

1. Check Mouseion API status first
2. Coordinate breaking changes
3. Update API client when Mouseion changes

## Community Standards

- Be respectful and constructive
- Focus on technical merit
- Assume good intent
- See [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)

## AI-Enabled Development

This project uses AI-enabled coding (Claude Code). All AI-generated code is reviewed before merging. Errors are possible but caught through review and testing.

## Support Development

This project is and always will be free and open source. Development is supported by:
- Code contributions (preferred)
- Bug reports and testing
- Documentation improvements
- Optional financial support: [GitHub Sponsors](https://github.com/sponsors/forkwright)

Funds support server costs, domains, development tools, and community infrastructure.

No pressure, no expectations - all contributions valued equally.

## License

GPL-3.0. By contributing, you agree your contributions will be licensed under GPL-3.0.

---

Questions? Open a GitHub Discussion or issue.
