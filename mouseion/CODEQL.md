# CodeQL Configuration

Mouseion uses GitHub CodeQL for security and quality scanning.

## Configured Languages

- **C#** - Primary language, scans all .NET source code

## Important: Disable Auto-Detected JavaScript Analysis

GitHub may auto-detect and attempt to scan JavaScript/TypeScript even though this is a .NET project with no JS/TS source files. This causes false errors.

### Fix Once (Repository Settings)

1. Navigate to: **Settings** → **Security** → **Code security and analysis**
2. Under **CodeQL analysis**, click **Configure**
3. Ensure only **C#** is enabled
4. Disable **JavaScript/TypeScript** if present

This persists across all future scans and prevents spurious "No JavaScript/TypeScript code found" errors.

## Workflow Configuration

- **Workflow**: `.github/workflows/codeql.yml`
- **Config**: `.github/codeql/codeql-config.yml`
- **Triggers**: Push to main/develop, PRs, weekly schedule (Monday 6am UTC)
- **Query suite**: security-and-quality

## Excluded Paths

Build artifacts and tooling directories are excluded:
- bin/, obj/, out/
- .git/, .github/, .vscode/, .vs/
- coverage/, TestResults/
- node_modules/, *.min.js, *.min.css

## Troubleshooting

**Error: "No JavaScript/TypeScript code found"**
- This indicates GitHub's auto-detection is running JavaScript analysis
- Follow the "Disable Auto-Detected JavaScript Analysis" steps above
- Our workflow only scans C# - this error is from GitHub's separate auto-config

**Workflow fails to run**
- Check that `.github/codeql/codeql-config.yml` exists
- Verify language matrix in workflow has only `['csharp']`
- Ensure .NET SDK version matches project requirements
