#!/usr/bin/env bash
# 잘못된 URL, 없는 번호, 수정·삭제 시도가 모두 거절되는지 확인한다.
#
# 사용법: scripts/check-rules.sh [서비스 주소]
#   예)   scripts/check-rules.sh http://localhost:8080
#         scripts/check-rules.sh https://배포주소
# 자기 주소 단축 거절은 서버의 PUBLIC_ORIGIN이 [서비스 주소]와 같을 때 확인된다.
set -euo pipefail

BASE="${1:-http://localhost:8080}"
BASE="${BASE%/}"
pass=0
fail=0

check() { # 설명, 기대값, 실제값
    if [[ "$3" == "$2" ]]; then
        pass=$((pass + 1)); printf '  통과  %-46s %s\n' "$1" "$3"
    else
        fail=$((fail + 1)); printf '  실패  %-46s %s (기대: %s)\n' "$1" "$3" "$2"
    fi
}

create() { # JSON 본문 → "상태코드"
    curl -sS -o /dev/null -w '%{http_code}' -H 'content-type: application/json' --data "$1" "$BASE/api/links"
}

echo "대상: $BASE"

echo "[잘못된 URL은 400으로 거절]"
long="https://example.org/$(printf 'a%.0s' $(seq 1 2040))"
for url in "javascript:alert(1)" "ftp://example.org" "naver.com" "https:naver.com" "" "   " \
           "https://exa mple.com" "https://" "$long" "$BASE/1"; do
    label="$url"; [[ ${#label} -gt 40 ]] && label="${label:0:30}…(${#url}자)"
    check "\"$label\"" 400 "$(create "{\"url\":\"$url\"}")"
done

echo "[없는 번호·번호가 아닌 주소는 404]"
for path in /999999999 /0 /01 /abc /1/ /a/b; do
    check "GET $path" 404 "$(curl -sS -o /dev/null -w '%{http_code}' "$BASE$path")"
done

echo "[만든 뒤에는 바꾸거나 지울 수 없음]"
original="https://example.org/immutable/$(date +%s)"
id="$(curl -sS -H 'content-type: application/json' --data "{\"url\":\"$original\"}" "$BASE/api/links")"
echo "  준비  /$id → $original"
for method in PUT PATCH DELETE; do
    check "$method /api/links" 405 "$(curl -sS -o /dev/null -w '%{http_code}' -X "$method" \
        -H 'content-type: application/json' --data "{\"url\":\"https://evil.example\"}" "$BASE/api/links")"
done
for method in POST PUT PATCH DELETE; do
    check "$method /$id" 405 "$(curl -sS -o /dev/null -w '%{http_code}' -X "$method" \
        -H 'content-type: application/json' --data "{\"url\":\"https://evil.example\"}" "$BASE/$id")"
done
other="$(curl -sS -H 'content-type: application/json' \
    --data "{\"url\":\"https://evil.example\",\"id\":$id}" "$BASE/api/links")"
if [[ "$other" != "$id" ]]; then
    check "번호를 지정한 생성 요청은 새 번호를 받음" "새 번호" "새 번호"
else
    check "번호를 지정한 생성 요청은 새 번호를 받음" "새 번호" "같은 번호 $other"
fi
check "모든 시도 뒤에도 /$id 는 원래 주소로 이동" "302 $original" \
    "$(curl -sS -o /dev/null -w '%{http_code} %{redirect_url}' "$BASE/$id")"

echo "통과 $pass, 실패 $fail"
[[ "$fail" == 0 ]] && echo "결과: 통과" || { echo "결과: 실패"; exit 1; }
