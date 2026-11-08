#!/usr/bin/env bash
# 서버나 DB를 다시 시작(재배포)해도 예전 단축 주소가 그대로 동작하는지 확인한다.
#
# 사용법:
#   1) scripts/check-persistence.sh [서비스 주소] create   # 확인용 단축 주소를 만들고 기록
#   2) 서버·DB를 재시작하거나 재배포
#   3) scripts/check-persistence.sh [서비스 주소] verify   # 기록한 주소가 그대로 이동하는지 확인
set -euo pipefail

BASE="${1:-http://localhost:8080}"
BASE="${BASE%/}"
MODE="${2:-}"
RECORD=".persistence-check"

case "$MODE" in
create)
    original="https://example.org/persistence/$(date +%Y%m%d%H%M%S)?check=restart#kept"
    id="$(curl -sS -H 'content-type: application/json' --data "{\"url\":\"$original\"}" "$BASE/api/links")"
    [[ "$id" =~ ^[1-9][0-9]*$ ]] || { echo "생성 실패: $id"; exit 1; }
    echo "$BASE $id $original" > "$RECORD"
    echo "만듦: $BASE/$id → $original"
    echo "이제 서버·DB를 재시작하거나 재배포한 뒤 'verify'로 다시 실행하세요."
    ;;
verify)
    [[ -f "$RECORD" ]] || { echo "먼저 create를 실행하세요."; exit 1; }
    read -r base id original < "$RECORD"
    got="$(curl -sS -o /dev/null -w '%{http_code} %{redirect_url}' "$base/$id")"
    echo "확인: $base/$id → $got"
    if [[ "$got" == "302 $original" ]]; then echo "결과: 통과"; else echo "결과: 실패 (기대: 302 $original)"; exit 1; fi
    ;;
*)
    echo "사용법: $0 [서비스 주소] create|verify"; exit 1 ;;
esac
