#!/bin/bash
# Manually scan repository for secrets
# Usage: ./scripts/scan-secrets.sh [--all|--staged]

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Check for gitleaks
if ! command -v gitleaks &> /dev/null; then
    echo "Error: gitleaks not found."
    echo "Install: brew install gitleaks (macOS) or choco install gitleaks (Windows)"
    exit 1
fi

MODE="${1:---all}"

echo "Scanning for secrets..."
echo ""

case "$MODE" in
    --staged)
        echo "Mode: Staged changes only"
        gitleaks protect --staged --config="$REPO_ROOT/.gitleaks.toml" --verbose
        ;;
    --all|*)
        echo "Mode: Full repository scan"
        gitleaks detect --source="$REPO_ROOT" --config="$REPO_ROOT/.gitleaks.toml" --verbose
        ;;
esac

STATUS=$?

echo ""
if [ $STATUS -eq 0 ]; then
    echo "✓ No secrets detected."
else
    echo "✗ Secrets detected! Review and remove before committing."
fi

exit $STATUS
