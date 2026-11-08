#!/usr/bin/env bash
# Cloudflare에 올릴 화면 파일(cloudflare/public)을 만든다.
#
# 1) dx bundle로 화면(WASM·JS·CSS)과 서버를 함께 빌드한다.
# 2) 그 서버를 잠깐 띄워 첫 화면 HTML을 받아 둔다. 이 HTML은 같은 빌드의 WASM과
#    짝이 맞아야 해서(파일 이름의 해시가 빌드마다 다르다), 같은 빌드로 만들어야 한다.
# 3) 화면 파일과 첫 화면 HTML을 cloudflare/public에 모은다.
#
# 서버를 띄우려면 DB가 필요하므로 먼저 `docker compose up -d`와 `.env`를 준비한다.
set -euo pipefail
cd "$(dirname "$0")/.."

BUNDLE=target/dx/short-url/release/web
OUT=cloudflare/public
PORT=18080

dx bundle --web --release

set -a; . ./.env; set +a
IP=127.0.0.1 PORT=$PORT "$BUNDLE/server" > /dev/null 2>&1 &
SERVER_PID=$!
trap 'kill $SERVER_PID 2>/dev/null || true' EXIT

for _ in $(seq 1 30); do
    curl -sf -o /dev/null "http://127.0.0.1:$PORT/" && break
    sleep 1
done

rm -rf "$OUT"
cp -R "$BUNDLE/public" "$OUT"
curl -sf "http://127.0.0.1:$PORT/" > "$OUT/index.html"

echo "만듦: $OUT"
find "$OUT" -type f | sort
