#!/bin/bash
# Line counts of the port, main..HEAD in the fork. Buckets per file (renames count under the new path):
#   lock      pnpm-lock.yaml (left out)
#   generated any path with _generated
#   tests     *.test.ts(x), testFixtures/, packages/*/test/, apps/server/integration/, apps/server/scripts/acp-mock-agent.ts
#   docs      *.md
#   check     scripts/anyagent-port-check.ts
#   product   everything else (hand-written code and config)
: "${T3CODE_DIR:?Set T3CODE_DIR to the T3 Code fork}"
cd "$T3CODE_DIR" || exit 1
git diff --shortstat main..HEAD
git diff --numstat main..HEAD | awk -F'\t' '
$1 == "-" { next }
{
  f = $3; b = "product"
  gsub(/\{[^}]* => /, "", f); gsub(/\}/, "", f)   # a rename "{old => new}" counts under the new path
  if (f ~ /pnpm-lock\.yaml/) b = "lock"
  else if (f ~ /_generated/) b = "generated"
  else if (f ~ /\.test\.tsx?$/ || f ~ /testFixtures\// || f ~ /^packages\/[^\/]+\/test\// || f ~ /^apps\/server\/integration\// || f ~ /acp-mock-agent\.ts$/) b = "tests"
  else if (f ~ /\.md$/) b = "docs"
  else if (f ~ /scripts\/anyagent-port-check\.ts$/) b = "check"
  add[b] += $1; del[b] += $2
  if (b == "product" && f ~ /provider\/anyagent\//) anyadd += $1
}
END {
  n = split("product generated tests docs check lock", order, " ")
  for (i = 1; i <= n; i++) printf "%-10s added %6d  deleted %6d\n", order[i], add[order[i]], del[order[i]]
  printf "product added under provider/anyagent/: %d; elsewhere: %d\n", anyadd, add["product"] - anyadd
}'
echo "provider/anyagent non-test (wc -l):"; ls apps/server/src/provider/anyagent/*.ts | grep -v '\.test\.ts$' | xargs wc -l | tail -1
