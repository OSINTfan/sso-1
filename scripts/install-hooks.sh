#!/bin/bash
# Install git hooks for secret scanning
# Works on Linux, macOS, Git Bash on Windows, or WSL

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
HOOKS_DIR="$REPO_ROOT/.git/hooks"

echo "Installing pre-commit hooks for SSO-1 Oracle..."

# Check if .git exists
if [ ! -d "$REPO_ROOT/.git" ]; then
    echo "Error: Not a git repository. Run 'git init' first."
    exit 1
fi

# Create hooks directory if it doesn't exist
mkdir -p "$HOOKS_DIR"

# Option 1: Use pre-commit framework (recommended)
if command -v pre-commit &> /dev/null; then
    echo "Found pre-commit. Installing hooks via framework..."
    cd "$REPO_ROOT"
    pre-commit install
    echo "Done! Hooks installed via pre-commit framework."
    echo "Run 'pre-commit run --all-files' to scan existing files."
    exit 0
fi

# Option 2: Use gitleaks directly
if command -v gitleaks &> /dev/null; then
    echo "Found gitleaks. Installing direct hook..."
    cat > "$HOOKS_DIR/pre-commit" << 'EOF'
#!/bin/bash
# Pre-commit hook: scan for secrets using gitleaks

echo "Scanning for secrets..."

# Get repo root
REPO_ROOT="$(git rev-parse --show-toplevel)"

# Run gitleaks on staged changes
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
EOF
    chmod +x "$HOOKS_DIR/pre-commit"
    echo "Done! Hook installed at .git/hooks/pre-commit"
    exit 0
fi

# Neither tool found
echo ""
echo "Neither 'pre-commit' nor 'gitleaks' found."
echo ""
echo "Install one of the following:"
echo ""
echo "  Option 1 (recommended): pre-commit framework"
echo "    pip install pre-commit"
echo "    pre-commit install"
echo ""
echo "  Option 2: gitleaks standalone"
echo "    brew install gitleaks        # macOS"
echo "    choco install gitleaks       # Windows"
echo "    go install github.com/gitleaks/gitleaks/v8@latest  # Go"
echo ""
exit 1
