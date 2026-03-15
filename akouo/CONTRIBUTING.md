# Contributing

## Setup

- **Web:** `cd web && npm install && npm run dev`
- **Android:** `cd android && ./gradlew build`
- **Backend:** [Mouseion](https://github.com/forkwright/mouseion) instance (or use MSW mocks for web dev)

## Workflow

1. Branch from `main`: `git checkout -b feat/your-feature`
2. Commit with conventional format: `feat(scope): description`
3. Push and open PR targeting `main`
4. CI runs automatically (lint, build, tests)
5. Squash merge on approval

## Specs

Development is spec-driven. See `specs/` for active specifications.
New features should reference or create a spec. Spec template: `specs/TEMPLATE.md`.

## Standards

- **Commits:** Conventional format (`feat`, `fix`, `docs`, `refactor`, `perf`, `test`, `chore`)
- **PRs:** Descriptive title, context in body, no placeholder code
- **Tests:** Required for new features
- **Kotlin:** ktlint
- **TypeScript:** ESLint + Prettier

## License

GPL-3.0. Contributions are licensed under the same terms.
