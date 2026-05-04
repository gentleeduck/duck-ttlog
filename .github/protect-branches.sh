#!/usr/bin/env bash
# Apply branch protection to gentleeduck/duck-ttlog.
# Idempotent: re-run safely.
set -euo pipefail
REPO="${REPO:-gentleeduck/duck-ttlog}"
BRANCH="${BRANCH:-master}"
DEV_BRANCH="${DEV_BRANCH:-dev}"

apply() {
  local b="$1"; local strict="${2:-false}"
  echo ">> $REPO : $b"
  gh api -X PUT "/repos/$REPO/branches/$b/protection" -H "Accept: application/vnd.github+json" --input - <<JSON >/dev/null
{
  "required_status_checks": { "strict": $strict, "contexts": [] },
  "enforce_admins": false,
  "required_pull_request_reviews": {
    "required_approving_review_count": 0,
    "dismiss_stale_reviews": true,
    "require_code_owner_reviews": false
  },
  "restrictions": null,
  "allow_force_pushes": false,
  "allow_deletions": false,
  "required_linear_history": true,
  "required_conversation_resolution": true
}
JSON
}

apply "$BRANCH" true
apply "$DEV_BRANCH" false 2>/dev/null || echo "(dev branch may not exist yet)"
echo "done"
