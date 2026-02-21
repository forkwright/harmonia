# Contributing

## Setup

```bash
dotnet build Mouseion.sln
dotnet test
```

Requires .NET 10 SDK.

## Workflow

1. Branch from `main`: `git checkout -b feat/your-feature`
2. Conventional commits: `feat(scope): description`
3. PR targeting `main`, squash merge
4. CI runs automatically (build, test, lint, format)

## Specs

Development is spec-driven. See `specs/` for active work. New features should reference or create a spec.

## Standards

- **Commits:** `feat`, `fix`, `docs`, `refactor`, `test`, `chore`
- **Tests:** Required for new features
- **Formatting:** `dotnet format` must pass
- **No AI attribution** in commits or PRs

## License

GPL-3.0. Contributions licensed under the same terms.
