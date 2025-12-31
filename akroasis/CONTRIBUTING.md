# Contributing to Akroasis

Unified media player for audiobooks, ebooks, and music with bit-perfect audio playback.

## Project Status

🚧 **Phase 0: Research & Foundation** - Active Development

Project is in early development. Contribution guidelines will mature as the codebase grows.

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

**1. Create feature branch**
```bash
git checkout main
git pull origin main
git checkout -b feature/your-feature-name
```

**2. Follow development standards**
- Conventional commits: `feat(scope): description`
- No placeholder code
- Test changes before committing
- Match existing code patterns

**3. Submit pull request**
- Target `main` branch
- Include clear description
- Reference related issues
- Ensure CI passes

**4. User reviews and merges**

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
