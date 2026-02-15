#!/usr/bin/env bash
# clean-fixtures.sh — Remove all fixture data created by seed-fixtures.sh.
# Usage: ./scripts/clean-fixtures.sh
set -euo pipefail

DRONES=("auth-service" "frontend-revamp" "db-migration")

echo "Cleaning fixture data..."

for drone in "${DRONES[@]}"; do
  rm -rf ".hive/drones/$drone"
  rm -rf "$HOME/.claude/tasks/$drone"
  rm -rf "$HOME/.claude/teams/$drone"
  echo "  Removed $drone"
done

echo "Done!"
