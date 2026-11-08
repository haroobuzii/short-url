#!/usr/bin/env bash
# 여러 단축 주소를 한꺼번에 만들어도 번호가 겹치거나 덮어써지지 않는지 확인한다.
#
# 사용법: scripts/check-concurrency.sh [서비스 주소] [개수]
#   예)   scripts/check-concurrency.sh http://localhost:8080 50
#         scripts/check-concurrency.sh https://배포주소 50
set -euo pipefail

BASE="${1:-http://localhost:8080}"
BASE="${BASE%/}"
COUNT="${2:-50}"
RUN="$(date +%Y%m%d%H%M%S)-$$"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

expected_url() { echo "https://example.org/concurrency/$RUN/$1?n=$1#item-$1"; }

create_one() {
    local i="$1" url status
    url="$(expected_url "$i")"
    status="$(curl -sS -o "$WORK/body-$i" -w '%{http_code}' \
        -H 'content-type: application/json' \
        --data "{\"url\":\"$url\"}" \
        "$BASE/api/links")"
    echo "$status $(cat "$WORK/body-$i")" > "$WORK/result-$i"
}
export -f create_one expected_url
export BASE RUN WORK

echo "대상: $BASE"
echo "요청: 단축 주소 $COUNT개를 동시에 생성"
seq 1 "$COUNT" | xargs -P "$COUNT" -I{} bash -c 'create_one {}'

ok=0
ids=()
for i in $(seq 1 "$COUNT"); do
    read -r status id < "$WORK/result-$i"
    if [[ "$status" == 200 && "$id" =~ ^[1-9][0-9]*$ ]]; then
        ok=$((ok + 1))
        ids+=("$id")
        echo "$i $id" >> "$WORK/pairs"
    else
        echo "  실패: $i번 요청 → HTTP $status $id"
    fi
done
distinct="$(printf '%s\n' "${ids[@]}" | sort -u | wc -l | tr -d ' ')"

redirect_ok=0
while read -r i id; do
    got="$(curl -sS -o /dev/null -w '%{http_code} %{redirect_url}' "$BASE/$id")"
    want="302 $(expected_url "$i")"
    if [[ "$got" == "$want" ]]; then
        redirect_ok=$((redirect_ok + 1))
    else
        echo "  잘못된 이동: /$id → $got (기대: $want)"
    fi
done < "$WORK/pairs"

range="$(printf '%s\n' "${ids[@]}" | sort -n | sed -n '1p;$p' | paste -sd '~' -)"
echo "생성 성공:        $ok / $COUNT"
echo "서로 다른 번호:   $distinct / $COUNT  (번호 범위 $range)"
echo "자기 원본으로 이동: $redirect_ok / $COUNT"

if [[ "$ok" == "$COUNT" && "$distinct" == "$COUNT" && "$redirect_ok" == "$COUNT" ]]; then
    echo "결과: 통과"
else
    echo "결과: 실패"
    exit 1
fi
