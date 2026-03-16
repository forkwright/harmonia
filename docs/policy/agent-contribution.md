# Agent contribution policy

## Rule 1: all automated agent commits go through PRs

No agent pushes directly to main. Every automated commit opens a PR and must pass CI before merging.

**Exceptions:** None. Branch protection enforces this.

## Rule 2: commit scope limits

| Agent Role | Max files per commit | Max LOC per commit |
|-----------|---------------------|-------------------|
| Coder (sub-agent) | 10 | 500 |
| Reviewer | 0 (review only) | 0 |
| Orchestrator (Syn) | 20 | 1000 |
| Dependabot | Unlimited (deps only) | Unlimited |

Commits exceeding scope limits are split into multiple PRs.

## Rule 3: quality gates

All agent PRs must pass:
- [ ] Mouseion: `dotnet build --configuration Release && dotnet test --configuration Release && dotnet format --verify-no-changes`
- [ ] Akouo web: `npm run lint && npm run build && npx vitest run`
- [ ] Akouo android: `./gradlew build test`
- [ ] Commit message format (`commitlint`)
- [ ] No secrets in diff

## Rule 4: review requirements

| Change Type | Review Required |
|-------------|----------------|
| Runtime source | Human approval |
| UI source | Human approval |
| Documentation | Auto-merge after CI |
| Dependency updates | Auto-merge if patch, human if minor/major |
| Config/tooling | Human approval |

## Rule 5: attribution

All commits use the operator's identity (`forkwright <noreply@forkwright.dev>`). No `Co-authored-by: Claude` or agent attribution in public history. Agents are tools, not authors.

## Rule 6: branching

- **Branch naming:** `<type>/<description>` (e.g., `feat/audiobook-chapters`, `fix/gapless-gap`)
- **One concern per branch:** no mixing features
- **Always squash merge:** no merge commits
- **Delete branch after merge**
