# Contributing to Mouseion

Mouseion is a solo project with AI pair programming. Contributions are welcome!

## How to Contribute

**Preferred contributions:**
- Bug reports with reproduction steps
- Feature suggestions with use cases
- Code contributions via pull requests
- Documentation improvements
- Testing and feedback

## Development Setup

```bash
# Clone and build
git clone https://github.com/forkwright/mouseion.git
cd mouseion
dotnet build Mouseion.sln

# Run locally
dotnet run --project src/Mouseion.Host
```

## Pull Request Process

1. Fork the repository
2. Create a feature branch (`feature/your-feature` or `fix/your-bugfix`)
3. Follow conventional commit format: `type(scope): description`
4. Ensure tests pass and no new warnings introduced
5. Submit PR targeting `main` branch

## Code Standards

- Match existing codebase patterns
- Self-documenting code (minimal comments)
- No placeholder code or TODOs in PRs
- Zero compiler warnings for new code

## Conventional Commits

Format: `type(scope): description`

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation only
- `refactor`: Code restructuring (no behavior change)
- `test`: Test additions/changes
- `chore`: Build, deps, config

**Examples:**
- `feat(audiobook): add narrator matching logic`
- `fix(metadata): handle API timeout gracefully`
- `refactor(database): extract MediaItem base class`

## Community Standards

- Respectful, inclusive communication
- Focus on technical merit
- Assume good faith
- Help others learn

## Support Development

This project is and always will be free and open source. Development is supported by:
- Code contributions (preferred)
- Bug reports and testing
- Documentation improvements
- Optional financial support: [GitHub Sponsors](https://github.com/sponsors/forkwright)

Funds support server costs, domains, development tools, and community infrastructure.

No pressure, no expectations - all contributions valued equally.

## License

By contributing, you agree that your contributions will be licensed under GPL-3.0.
