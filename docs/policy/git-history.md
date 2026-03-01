# Git History Policy

## Principles

- Git history is a first-class artifact. It should be readable, meaningful, and useful for future maintainers.
- Every commit message tells a story: what changed and why.
- Squash merge on PR — branch preserves detailed history, main gets a clean narrative.

## Rules

1. **Conventional commits.** `type(scope): description` format. Always.
2. **Present tense, imperative mood.** "Add X" not "Added X" or "Adds X."
3. **First line ≤72 characters.** Body wraps at 80.
4. **One logical change per commit.** If you need "and" in the commit message, it's two commits.
5. **No merge commits on main.** Squash merge only.
6. **Delete branches after merge.** Dead branches are noise.

## Rewriting History

- **Before push:** Rebase, squash, amend freely. Your branch, your rules.
- **After push:** Only rebase/force-push your own feature branches. Never force-push main.
- **After merge:** History is immutable. If something is wrong, fix forward with a new commit.
