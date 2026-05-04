#!/usr/bin/env bash
# Apply the dmc label scheme to gentleeduck/duck-mc.
# Idempotent: existing labels are updated; new ones created.
#
# Usage:
#   bash .github/labels.sh
#   REPO=other/repo bash .github/labels.sh

set -euo pipefail

REPO="${REPO:-gentleeduck/duck-ttlog}"

# label name | color (hex, no #) | description
labels=(
  # Type
  "bug 🐛|D73A4A|Something is broken"
  "feat ✨|0E8A16|New capability or surface"
  "fix 🔧|E99695|Bug fix"
  "perf ⚡|FBCA04|Speedup or memory improvement"
  "refactor ♻️|1D76DB|Code change without behaviour change"
  "docs 📚|0075CA|Docs only"
  "test ✅|C2E0C6|Test only"
  "chore 🧹|EDEDED|Routine maintenance"
  "style 🎨|F9D0C4|Formatting / whitespace"
  "build 📦|C5DEF5|Build system / packaging"
  "ci 🤖|BFD4F2|CI/CD only"
  "release 🚀|5319E7|Release / version bump"
  "revert ⏪|B60205|Revert a prior change"


  # Area
  "area: cli 💻|6F42C1|CLI surface"
  "area: api 🔌|6F42C1|Public TS / Rust API"
  "area: cache 💾|6F42C1|File / math cache"
  "area: math 🧮|6F42C1|KaTeX / MathML"
  "area: pretty-code 🌈|6F42C1|Syntax highlighting"
  "area: mermaid 🧜|6F42C1|Mermaid diagrams"
  "area: jsx ⚛️|6F42C1|JSX parsing / emission"
  "area: gfm 📋|6F42C1|GFM tables / strike / autolinks"
  "area: examples 🧪|6F42C1|Example apps"
  "area: bench 📊|6F42C1|Benchmarks"

  # Status / triage
  "status: triage 🔍|E4E669|Needs maintainer review"
  "status: blocked 🚧|B60205|Blocked on external work"
  "status: in-progress 🏗|FBCA04|Actively being worked"
  "status: stale 🥱|CCCCCC|Stale, may be auto-closed"
  "status: needs-repro 🔁|D93F0B|Cannot proceed without reproduction"
  "needs: design 💡|D4C5F9|Needs design discussion"

  # Priority
  "priority: critical 🔥|B60205|Production-impacting"
  "priority: high 🔴|D93F0B|Address soon"
  "priority: medium 🟡|FBCA04|Normal queue"
  "priority: low 🟢|0E8A16|Whenever"

  # Difficulty
  "good first issue 🌱|7057FF|Approachable for newcomers"
  "help wanted 🙋|008672|Community help welcomed"
  "hacktoberfest 🎃|FF8C00|Open for Hacktoberfest"

  # Resolution
  "wontfix 🚫|FFFFFF|Will not be fixed"
  "duplicate 👯|CFD3D7|Duplicate of another issue or PR"
  "invalid ❓|E4E669|Not actionable"
  "question 💬|D876E3|Discussion / clarification"

  # Dependencies
  "dependencies 📦|0366D6|Dependency update"
  "rust 🦀|DEA584|Rust dep"
  "javascript 🟨|F1E05A|JS / TS dep"
  "github-actions 🤖|2B7489|GH Actions dep"

  # Security
  "security 🔒|B60205|Security-sensitive"

  # Breaking
  "breaking 💥|B60205|Breaking change"
)

upsert() {
  local name="$1"; local color="$2"; local desc="$3"
  if gh label list --repo "$REPO" --limit 200 --json name -q '.[].name' | grep -Fxq "$name"; then
    gh label edit "$name" --repo "$REPO" --color "$color" --description "$desc" >/dev/null
    echo "updated  $name"
  else
    gh label create "$name" --repo "$REPO" --color "$color" --description "$desc" >/dev/null
    echo "created  $name"
  fi
}

for entry in "${labels[@]}"; do
  IFS='|' read -r name color desc <<<"$entry"
  upsert "$name" "$color" "$desc"
done

echo "done"
