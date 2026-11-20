#!/usr/bin/env bash
# F022 acceptance checks on a running stack (docker compose -f deploy/compose.yaml up -d).
set -euo pipefail

BASE_URL="${BASE_URL:-http://localhost:${HTTP_PORT:-8000}}"
COMPOSE=(docker compose -f "$(dirname "$0")/compose.yaml")
failures=0

check() {
  local scenario="$1" description="$2"
  shift 2
  if "$@"; then
    echo "ok   $scenario $description"
  else
    echo "FAIL $scenario $description"
    failures=$((failures + 1))
  fi
}

status_of() { curl -s -o /dev/null -w '%{http_code}' "$BASE_URL$1"; }

# Wait for the API behind the proxy (database migrations run at start-up).
for _ in $(seq 1 30); do
  [ "$(status_of /api/health)" = "200" ] && break
  sleep 2
done

health="$(curl -s "$BASE_URL/api/health")"
check F022-S01 "API healthy through the proxy" grep -q '"database":"ok"' <<<"$health"
check F022-S01 "home page rendered on the server (F002-S01)" bash -c "curl -s '$BASE_URL/' | grep -Eq 'La Petite Grange[[:space:]]*</h1>'"
check F002-S01 "English page rendered on the server" bash -c "curl -s '$BASE_URL/en/the-cottage' | grep -q 'The cottage'"
check F002-S03 "unknown page is 404" test "$(status_of /page-inconnue)" = "404"
check F022-S02 "unknown page is 404" test "$(status_of /does-not-exist)" = "404"

for service in api front; do
  # User declared by the image (the API image has no shell to run "id").
  user="$(docker inspect --format '{{.Config.User}}' "$("${COMPOSE[@]}" ps -q "$service")")"
  check F022-S03 "$service does not run as root (user: $user)" test "$user" != "0" -a "$user" != "root" -a -n "$user"
done

# Services with a host port mapping ("->" in the Ports column).
published="$("${COMPOSE[@]}" ps --format '{{.Service}} {{.Ports}}' | grep -- '->' | cut -d' ' -f1 | sort -u | tr '\n' ' ')"
check F022-S04 "only the proxy publishes a port (published: $published)" test "$published" = "caddy "

if [ "$failures" -gt 0 ]; then
  echo "$failures check(s) failed"
  exit 1
fi
echo "All F022 checks passed."
