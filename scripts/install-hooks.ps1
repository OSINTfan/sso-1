# Install git hooks for secret scanning (Windows PowerShell)
# Run: .\scripts\install-hooks.ps1

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = Split-Path -Parent $ScriptDir
$HooksDir = Join-Path $RepoRoot ".git\hooks"

Write-Host "Installing pre-commit hooks for SSO-1 Oracle..." -ForegroundColor Cyan

# Check if .git exists
if (-not (Test-Path (Join-Path $RepoRoot ".git"))) {
    Write-Host "Error: Not a git repository. Run 'git init' first." -ForegroundColor Red
    exit 1
}

# Create hooks directory if it doesn't exist
if (-not (Test-Path $HooksDir)) {
    New-Item -ItemType Directory -Path $HooksDir -Force | Out-Null
}

# Option 1: Use pre-commit framework (recommended)
$precommit = Get-Command pre-commit -ErrorAction SilentlyContinue
if ($precommit) {
    Write-Host "Found pre-commit. Installing hooks via framework..." -ForegroundColor Green
    Push-Location $RepoRoot
    pre-commit install
    Pop-Location
    Write-Host "Done! Hooks installed via pre-commit framework." -ForegroundColor Green
    Write-Host "Run 'pre-commit run --all-files' to scan existing files."
    exit 0
}

# Option 2: Use gitleaks directly
$gitleaks = Get-Command gitleaks -ErrorAction SilentlyContinue
if ($gitleaks) {
    Write-Host "Found gitleaks. Installing direct hook..." -ForegroundColor Green

    $hookScript = @'
#!/bin/bash
# Pre-commit hook: scan for secrets using gitleaks

echo "Scanning for secrets..."

REPO_ROOT="$(git rev-parse --show-toplevel)"

gitleaks protect --staged --config="$REPO_ROOT/.gitleaks.toml" --verbose

if [ $? -ne 0 ]; then
    echo ""
    echo "ERROR: Secrets detected! Commit blocked."
    echo "Review the findings above and remove secrets before committing."
    echo ""
    echo "If this is a false positive, you can:"
    echo "  1. Add the pattern to .gitleaks.toml allowlist"
    echo "  2. Skip this check with: git commit --no-verify (not recommended)"
    exit 1
fi

echo "No secrets detected."
'@

    $hookPath = Join-Path $HooksDir "pre-commit"
    $hookScript | Out-File -FilePath $hookPath -Encoding utf8 -NoNewline
    Write-Host "Done! Hook installed at .git/hooks/pre-commit" -ForegroundColor Green
    exit 0
}

# Neither tool found
Write-Host ""
Write-Host "Neither 'pre-commit' nor 'gitleaks' found." -ForegroundColor Yellow
Write-Host ""
Write-Host "Install one of the following:"
Write-Host ""
Write-Host "  Option 1 (recommended): pre-commit framework" -ForegroundColor Cyan
Write-Host "    pip install pre-commit"
Write-Host "    pre-commit install"
Write-Host ""
Write-Host "  Option 2: gitleaks standalone" -ForegroundColor Cyan
Write-Host "    choco install gitleaks       # Chocolatey"
Write-Host "    scoop install gitleaks       # Scoop"
Write-Host "    winget install gitleaks      # WinGet"
Write-Host ""
exit 1
