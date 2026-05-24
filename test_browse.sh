#!/usr/bin/env bash
# test_browse.sh — Test all manga from browse/search results end-to-end
# Usage: ./test_browse.sh [source_wasm] [query]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
DEV="$SCRIPT_DIR/target/release/koma-source-dev"

WASM="${1:-}"
QUERY="${2:-}"

if [[ -z "$WASM" ]]; then
  echo "Usage: $0 <source.wasm> [search_query]" >&2
  exit 1
fi

PASS=0
FAIL=0
ERRORS=()

log() { echo -e "$@" >&2; }

test_manga() {
  local manga_id="$1"
  local manga_title="$2"
  local source_name="$3"

  # get_manga
  local manga_result
  manga_result=$("$DEV" run --op get_manga --request "{\"mangaId\":\"$manga_id\"}" "$WASM" 2>/dev/null)
  local manga_ok
  manga_ok=$(echo "$manga_result" | jq -r '.ok')
  if [[ "$manga_ok" != "true" ]]; then
    local err=$(echo "$manga_result" | jq -r '.error.code // "unknown"')
    local err_msg=$(echo "$manga_result" | jq -r '.error.message // ""')
    log "  ✗ get_manga FAIL ($err: $err_msg)"
    # Save raw output for diagnosis
    FAIL=$((FAIL + 1))
    ERRORS+=("[$source_name] $manga_title ($manga_id): get_manga → $err")
    return
  fi
  log "  ✓ get_manga"

  # get_chapters
  local chapters_result
  chapters_result=$("$DEV" run --op get_chapters --request "{\"mangaId\":\"$manga_id\"}" "$WASM" 2>/dev/null)
  local chapters_ok
  chapters_ok=$(echo "$chapters_result" | jq -r '.ok')
  if [[ "$chapters_ok" != "true" ]]; then
    local err=$(echo "$chapters_result" | jq -r '.error.code // "unknown"')
    log "  ✗ get_chapters FAIL ($err)"
    FAIL=$((FAIL + 1))
    ERRORS+=("[$source_name] $manga_title ($manga_id): get_chapters → $err")
    return
  fi
  local ch_count
  ch_count=$(echo "$chapters_result" | jq '.data.items | length')
  log "  ✓ get_chapters ($ch_count chapters)"

  if [[ "$ch_count" -eq 0 ]]; then
    log "  ⚠ no chapters, skipping get_pages"
    PASS=$((PASS + 1))
    return
  fi

  # get_pages for first chapter
  local first_ch_id
  first_ch_id=$(echo "$chapters_result" | jq -r '.data.items[0].id')
  local pages_result
  pages_result=$("$DEV" run --op get_pages --request "{\"chapterId\":\"$first_ch_id\"}" "$WASM" 2>/dev/null)
  local pages_ok
  pages_ok=$(echo "$pages_result" | jq -r '.ok')
  if [[ "$pages_ok" != "true" ]]; then
    local err=$(echo "$pages_result" | jq -r '.error.code // "unknown"')
    log "  ✗ get_pages FAIL ($err) [chapter: $first_ch_id]"
    FAIL=$((FAIL + 1))
    ERRORS+=("[$source_name] $manga_title ($manga_id): get_pages($first_ch_id) → $err")
    return
  fi
  local page_count
  page_count=$(echo "$pages_result" | jq '.data.pages | length')
  log "  ✓ get_pages ($page_count pages)"

  PASS=$((PASS + 1))
}

# Determine source name from wasm filename
SOURCE_NAME=$(basename "$WASM" .wasm | sed 's/koma_//;s/_source//')

# Get manga list: try search if query provided, otherwise try get_manga_list
MANGA_IDS=()
MANGA_TITLES=()

if [[ -n "$QUERY" ]]; then
  log "▸ Searching '$QUERY' on $SOURCE_NAME..."
  SEARCH_RESULT=$("$DEV" run --op search --request "{\"query\":\"$QUERY\",\"page\":1,\"limit\":20}" "$WASM" 2>/dev/null)
else
  log "▸ Getting manga list from $SOURCE_NAME..."
  SEARCH_RESULT=$("$DEV" run --op get_manga_list --request "{\"page\":1,\"limit\":20}" "$WASM" 2>/dev/null)
fi

SEARCH_OK=$(echo "$SEARCH_RESULT" | jq -r '.ok')
if [[ "$SEARCH_OK" != "true" ]]; then
  ERR=$(echo "$SEARCH_RESULT" | jq -r '.error.code // "unknown"')
  log "✗ Failed to get manga list: $ERR"
  exit 1
fi

COUNT=$(echo "$SEARCH_RESULT" | jq '.data.items | length')
log "▸ Found $COUNT manga, testing each..."
log ""

for i in $(seq 0 $((COUNT - 1))); do
  MID=$(echo "$SEARCH_RESULT" | jq -r ".data.items[$i].id")
  MTITLE=$(echo "$SEARCH_RESULT" | jq -r ".data.items[$i].title // \"untitled\"")
  log "[$((i+1))/$COUNT] $MTITLE"
  test_manga "$MID" "$MTITLE" "$SOURCE_NAME"
  log ""
  # Throttle to avoid upstream rate-limiting per-IP (especially behind a proxy node).
  sleep "${SLEEP_BETWEEN:-1}"
done

log "═══════════════════════════════"
log "Results: $PASS PASS / $FAIL FAIL / $COUNT total"
if [[ ${#ERRORS[@]} -gt 0 ]]; then
  log ""
  log "Failures:"
  for e in "${ERRORS[@]}"; do
    log "  • $e"
  done
fi
log "═══════════════════════════════"

exit $FAIL
