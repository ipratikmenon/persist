#!/usr/bin/env bash
# RLS acceptance gate — specs/module-05-portal.md §8
#
# Applies server/migrations/ to a throwaway database and runs
# server/tests/rls_test.sql against it. Exits non-zero on any failed assertion.
#
# This must pass before any portal endpoint is written.
#
# Usage:
#   server/scripts/test-rls.sh              # throwaway DB named persist_rls_test
#   RLS_TEST_DB=mydb server/scripts/test-rls.sh
#
# Requires a reachable PostgreSQL and permission to create databases.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SERVER_DIR="$(dirname "$SCRIPT_DIR")"

DB_NAME="${RLS_TEST_DB:-persist_rls_test}"
PSQL_OPTS=(--no-psqlrc --quiet -v ON_ERROR_STOP=1)

cleanup() {
    dropdb --if-exists "$DB_NAME" >/dev/null 2>&1 || true
}
trap cleanup EXIT

echo "==> Preparing database: $DB_NAME"
dropdb --if-exists "$DB_NAME" >/dev/null 2>&1 || true
createdb "$DB_NAME"

echo "==> Applying migrations"
for migration in "$SERVER_DIR"/migrations/*.sql; do
    echo "    $(basename "$migration")"
    psql "${PSQL_OPTS[@]}" -d "$DB_NAME" -f "$migration" >/dev/null
done

echo "==> Running RLS tests"
# Assertions report via RAISE NOTICE (stderr) and abort the script on failure,
# so psql's exit status is the verdict. Show the PASS/ERROR lines either way.
set +e
psql "${PSQL_OPTS[@]}" -d "$DB_NAME" -f "$SERVER_DIR/tests/rls_test.sql" 2>&1 \
    | grep -E 'NOTICE:|ERROR:' \
    | sed -e 's/^.*NOTICE:  /    /' -e 's/^.*ERROR:  /    FAILED: /'
status="${PIPESTATUS[0]}"
set -e

if [ "$status" -eq 0 ]; then
    echo "==> RLS GATE: PASS"
else
    echo "==> RLS GATE: FAIL"
fi
exit "$status"
