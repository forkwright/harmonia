# Claude Code Dispatch Protocol

> Template and guardrails for generating Claude Code task prompts.
> Syn references this document every time work is delegated to Claude Code sessions.

---

## Critical: Execution Framing

Claude Code sessions must **execute tasks**, not analyze prompts. Every prompt MUST
open with an imperative action directive. The model defaults to exploration and
commentary unless explicitly told to implement.

**Always start prompts with:**
```
You are an engineer. Implement the following task completely — write code, run
tests, commit, push, and open a PR. Do not summarize the task, audit the prompt,
or explain what you would do. Do the work.
```

**Key framing rules:**
- "Do what has been asked; nothing more, nothing less."
- "Avoid over-engineering. Only make changes that are directly requested."
- "Don't add features, refactor code, or make 'improvements' beyond what was asked."
- "Do not create files unless they're absolutely necessary."
- "Read and understand existing code before suggesting modifications."

**Avoid these anti-patterns that cause analysis-not-action:**
- "Here is the task..." (reads as a briefing doc to analyze)
- Long context sections before the action directive (model starts analyzing)
- "You should..." / "Consider..." (advisory, not imperative)
- "Implement..." / "Create..." / "Build..." / "Write..." (direct action verbs)
- Action directive FIRST, then context, then scope details

---

## Environment

- **Clone location:** `/home/ck/aletheia-ops/harmonia` on Metis
- **Working directory:** Claude Code sessions are opened in this directory

## Prompt Preamble

Every Claude Code prompt MUST start with the execution directive, then this setup block:

```
## Directive

You are an engineer. Implement this task completely — write the code, run the
tests, fix any issues, commit, push, and open a PR. Do not analyze or summarize
the prompt. Execute it.

## Setup

You are working in the Harmonia repository at /home/ck/aletheia-ops/harmonia.

Before doing anything:

1. Verify your clone is current:
   git fetch origin && git log --oneline -3 origin/main

2. Create a git worktree for your work (do NOT work on main directly):
   git worktree add ../worktrees/<branch-name> -b <branch-name> origin/main
   cd ../worktrees/<branch-name>

3. Do all your work in the worktree. When done:
   - Commit with conventional commit messages (feat:, fix:, refactor:, docs:, chore:)
   - Push the branch: git push origin <branch-name>
   - Create a PR: gh pr create --base main --title "<title>" --body "<body>"
   - Do NOT merge — Syn reviews and merges

4. Clean up the worktree when done:
   cd /home/ck/aletheia-ops/harmonia
   git worktree remove ../worktrees/<branch-name>
```

## Standards References

Every prompt MUST reference the relevant standards. Include this block:

```
## Standards

Read these before writing any code:

- docs/STANDARDS.md — master standards document, all languages
- .claude/rules/rust.md — Rust-specific rules (snafu, async, newtypes)
- .claude/rules/dotnet.md — C#/.NET rules (Dapper, DryIoc, async/await)
- .claude/rules/kotlin.md — Kotlin rules (Compose, Hilt, coroutines)

Key rules you MUST follow:
- Mouseion: dotnet build/test/format must pass
- Akroasis web: npm run lint && npm run build && npx vitest run
- Akroasis android: ./gradlew build test
- No AI attribution in commits or code
- Conventional commits: feat(scope):, fix(scope):, etc.
```

## Validation Gate

Every prompt MUST include this verification block at the end:

```
## Before Creating the PR

Run the relevant checks and fix any issues:

Mouseion (if touching mouseion/):
1. dotnet build mouseion/Mouseion.sln --configuration Release
2. dotnet test mouseion/ --configuration Release --no-build
3. dotnet format mouseion/Mouseion.sln --verify-no-changes

Akroasis web (if touching akroasis/web/):
1. cd akroasis/web && npm ci
2. npm run lint
3. npm run build
4. npx vitest run

Akroasis android (if touching akroasis/android/):
1. cd akroasis/android && ./gradlew build test

All:
- git diff --stat  (review your own changes — nothing unexpected)

If any check fails, fix it before creating the PR.

After the PR is created, respond with: PR: <url>
If no PR was created, respond with: PR: none — <reason>
```

## Branch Naming

Use descriptive prefixes:
- `feat/<feature-name>` — new functionality
- `fix/<bug-description>` — bug fixes
- `refactor/<scope>` — restructuring without behavior change
- `docs/<topic>` — documentation only
- `chore/<topic>` — deps, CI, tooling

## Prompt Structure Template

```markdown
# Task: <clear one-line description>

## Directive

You are an engineer. Implement this task completely — write the code, run the
tests, fix any issues, commit, push, and open a PR. Do not analyze or summarize
the prompt. Execute it.

## Setup
<preamble block from above — with specific branch name>

## Standards
<standards block from above — prune to relevant languages>

## Context
<what exists, what's already been decided, relevant commits/issues>

## Task
<exactly what to build/fix — imperative voice, specific files and behaviors>

## Acceptance Criteria
<numbered list of concrete, verifiable outcomes>

## Before Creating the PR
<validation gate block from above>
```

## Prompt Quality Checklist

Before dispatching a prompt, verify:

1. **Does it open with an action directive?** First sentence must be imperative.
2. **Is the task section in imperative voice?** "Create X" not "X should be created."
3. **Is context minimal?** Only what's needed to orient. Don't front-load analysis.
4. **Are acceptance criteria testable?** Each one should be pass/fail verifiable.
5. **Is there a clear deliverable?** PR URL, file path, or explicit output format.

## Parallel Session Coordination

When dispatching multiple Claude Code sessions simultaneously:
- Each session gets a UNIQUE branch name
- Branches should not touch overlapping files (merge conflicts)
- If overlap is unavoidable, sequence them
- Note in each prompt: "Other sessions may be running. Do not modify: <list of files owned by other sessions>"

## Post-Merge Checklist (for Syn)

After Claude Code creates PRs:
1. `git fetch origin && gh pr list`
2. Review each PR diff: `gh pr diff <number>`
3. Run checks on the branch
4. Fix any issues, push to the branch
5. Squash merge: `gh pr merge <number> --squash --subject "<conventional commit>" --delete-branch`
6. Pull main: `git pull --rebase`
7. Verify clean state
