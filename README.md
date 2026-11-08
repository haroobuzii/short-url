# 숫자형 단축 URL 서비스

## 배포 URL

- 서비스 URL: https://short-url.haroo.workers.dev
- 테스트 단축 URL: https://short-url.haroo.workers.dev/134
- 테스트 원본 URL: https://en.wikipedia.org/wiki/URL_redirection?action=view#HTTP_status_codes_3xx

누구나 로그인 없이 긴 URL을 `/숫자` 형태의 짧은 주소로 바꾸고, 그 주소로 원래 URL에 이동할 수 있다. 화면과 서버 모두 Rust [Dioxus](https://dioxuslabs.com) 0.7 fullstack 하나로 만들었다.

## 구성

```
브라우저 ──► Cloudflare Worker (https://short-url.haroo.workers.dev)
              ├─ 화면 파일(HTML·JS·WASM·CSS) → Worker가 직접 응답
              └─ /api/links, /숫자, 그 밖의 주소 → Railway로 그대로 전달
                   ▼
             Railway: Dioxus 서버 (생성 API, 302 이동, 404 페이지)
                   ▼
             Railway: PostgreSQL (연결 저장)
```

| 위치 | 역할 | 파일 |
|---|---|---|
| Cloudflare Workers | 웹 화면과 공개 입구. 화면 파일을 주고 나머지 요청을 Railway로 넘긴다. | `cloudflare/` |
| Railway 서비스 | Rust 백엔드. 생성 API(`POST /api/links`), `/숫자` 이동, 404 | `src/`, `Dockerfile` |
| Railway PostgreSQL | 번호와 원본 URL 저장 | `migrations/` |

브라우저에는 공개 주소 하나만 보이므로 CORS 설정이 필요 없고, Railway 주소나 DB 접속 정보는 브라우저 코드에 들어가지 않는다. 이렇게 고른 이유는 `docs/adr/0002-cloudflare-worker-proxies-to-railway.md`에 있다.

## 동작 규칙

### 번호 발급과 중복 방지

- 번호는 DB가 매긴다. `links.id`는 `BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY`이다.
  - PostgreSQL의 시퀀스는 동시에 들어온 요청에도 같은 값을 두 번 주지 않는다.
  - `PRIMARY KEY`가 같은 번호의 두 줄을 막는다.
  - `GENERATED ALWAYS`라서 요청에 번호를 넣어도 직접 지정할 수 없다.
- 번호는 1부터 커지며, 저장에 실패한 요청도 번호 하나를 쓰므로 중간에 빈 번호가 생길 수 있다. 한 번 나간 번호는 다시 쓰지 않는다.
- 같은 원본 URL을 여러 번 넣으면 매번 새 번호를 준다. 이유는 `docs/adr/0001-new-number-for-every-request.md`에 있다.
- 서버는 번호만 돌려주고, 화면이 지금 접속한 공개 주소 뒤에 `/번호`를 붙여 단축 주소를 보여 준다.

### 원본 URL 검사 (서버에서 수행)

앞뒤 공백을 지운 뒤 아래를 모두 만족해야 저장한다. 하나라도 어기면 400과 한국어 이유를 돌려준다.

- `http://` 또는 `https://`로 시작한다. (`javascript:`, `ftp://`, `naver.com`, `https:naver.com`은 거절)
- 호스트가 있고 URL 형식이 올바르다.
- 중간에 공백이나 줄바꿈이 없다.
- 2048자 이하이다. DB의 `CHECK` 제약으로도 한 번 더 막는다.
- 이 서비스의 공개 주소(`PUBLIC_ORIGIN`)를 가리키지 않는다.

통과하면 앞뒤 공백만 뺀 입력 그대로 저장한다.

### 이동

- `/숫자`로 들어오면 서버가 302 응답의 `Location`으로 원본 URL을 보낸다. 경로·쿼리·프래그먼트를 모두 유지한다.
  - 한글 같은 글자는 브라우저가 주소창 입력을 처리할 때와 같은 규칙(WHATWG URL)으로 퍼센트 인코딩해서 보낸다.
  - 브라우저가 이동을 기억해 두지 않도록 301 대신 302를 쓴다.
- 1 이상이고 앞에 0이 없는 숫자만 번호로 본다. `/0`, `/01`, `/abc`, 너무 큰 수, `/1/`, `/a/b`, 없는 번호는 모두 404 안내 페이지를 보여 준다.

### 변경 불가

- 연결을 수정·삭제하는 API가 없다. `/api/links`에 PUT·PATCH·DELETE, `/숫자`에 POST·PUT·PATCH·DELETE를 보내면 405가 된다.
- DB 트리거가 `links`의 UPDATE·DELETE·TRUNCATE를 거부한다. 서버 코드에 실수가 있어도 DB가 막는다.
- 만료나 번호 재사용도 없다.

## 로컬 실행

필요한 것: Rust, [Dioxus CLI](https://dioxuslabs.com/learn/0.7/getting_started) 0.7, Docker

1. 환경 변수 파일을 만든다. 들어 있는 값은 로컬 개발용이다.

   ```bash
   cp .env.example .env
   ```

2. Docker로 PostgreSQL을 띄운다. 표는 서버가 켜질 때 `migrations/`의 SQL로 자동으로 만든다.

   ```bash
   docker compose up -d
   ```

3. 앱을 실행하고 브라우저에서 http://localhost:8080 을 연다.

   ```bash
   dx serve --platform web
   ```

### 환경 변수

| 이름 | 쓰는 곳 | 뜻 |
|---|---|---|
| `DATABASE_URL` | Railway 서비스 | PostgreSQL 접속 주소 |
| `PUBLIC_ORIGIN` | Railway 서비스 | 이 서비스의 공개 주소. 이 주소를 가리키는 URL은 단축을 거절한다. 여러 개면 쉼표로 구분한다. |
| `BACKEND_ORIGIN` | Cloudflare Worker | 요청을 넘길 Railway 주소. `cloudflare/wrangler.jsonc`에 있다. |
| `IP`, `PORT` | Railway 서비스 | 서버가 받을 주소와 포트. `Dockerfile`에서 `0.0.0.0`, `8080`으로 정하고, Railway가 `PORT`를 넣어 준다. |

실제 비밀 값(Railway DB 비밀번호 등)은 저장소에 없다. `.env`는 git에서 제외하며, `.env.example`과 `docker-compose.yml`의 비밀번호는 로컬 개발 전용이다.

### 테스트

DB 테스트는 실제 PostgreSQL이 필요하다. 테스트마다 임시 DB를 새로 만들고 끝나면 지우므로, 개발용 데이터에는 영향이 없다.

```bash
docker compose up -d
set -a; . ./.env; set +a
cargo test --no-default-features --features server
```

## 배포

### Railway (백엔드와 DB)

1. Railway 프로젝트에 **PostgreSQL**을 추가한다.
2. 같은 프로젝트에 이 GitHub 저장소를 서비스로 추가한다. Railway가 `Dockerfile`로 빌드하고, push할 때마다 다시 배포한다.
3. 서비스의 Variables에 아래를 넣는다.
   - `DATABASE_URL` = `${{Postgres.DATABASE_URL}}` (Railway 참조 변수. 비밀번호를 복사하지 않는다.)
   - `PUBLIC_ORIGIN` = `https://short-url.haroo.workers.dev,https://short-url-production-da76.up.railway.app`
4. Settings → Networking → Generate Domain에서 포트 8080으로 공개 주소를 만든다.

### Cloudflare Workers (웹 화면과 공개 입구)

`cloudflare/wrangler.jsonc`의 `BACKEND_ORIGIN`을 Railway 주소로 맞춘 뒤 실행한다.

```bash
docker compose up -d                       # 첫 화면 HTML을 만들 때 서버를 잠깐 띄우므로 DB가 필요하다
scripts/build-cloudflare.sh                # cloudflare/public에 화면 파일을 만든다
cd cloudflare && npx wrangler@4 deploy     # 처음 한 번은 npx wrangler@4 login
```

`build-cloudflare.sh`는 `dx bundle`로 화면을 빌드하고, 같은 빌드의 서버를 잠깐 띄워 첫 화면 HTML을 받아 함께 넣는다. 화면 파일 이름에 빌드마다 다른 해시가 붙으므로 HTML과 WASM은 같은 빌드에서 나와야 한다.

### 라우팅

| 요청 | 처리 |
|---|---|
| `GET /`, `/assets/*` | Cloudflare가 `cloudflare/public`의 파일로 바로 응답 |
| `POST /api/links` | Worker → Railway. 400(거절) 또는 번호 |
| `GET /숫자` | Worker → Railway → 302. Worker는 이동을 따라가지 않고 그대로 브라우저에 전달 |
| 그 밖의 주소 | Worker → Railway → 404 안내 페이지 |

## 검증 시나리오

`scripts/`의 스크립트는 주소를 받아서 로컬과 배포 환경 모두에 쓸 수 있다.

```bash
scripts/check-concurrency.sh https://short-url.haroo.workers.dev 50   # 동시 생성
scripts/check-rules.sh https://short-url.haroo.workers.dev            # 잘못된 URL, 없는 번호, 수정 시도 거절
scripts/check-persistence.sh https://short-url.haroo.workers.dev create
# 서버를 재시작하거나 재배포한 뒤
scripts/check-persistence.sh https://short-url.haroo.workers.dev verify
```

### 공개 주소 결과 (2026-11-08, https://short-url.haroo.workers.dev)

| 과제 시나리오 | 결과 |
|---|---|
| 시크릿 창에서 생성·복사 | 쿠키나 로그인 없는 브라우저에서 화면을 열어 원본 URL을 넣고 엔터 → `/134` 생성, 복사 버튼 표시 |
| 다른 브라우저에서 이동 | `/134`를 열면 `https://en.wikipedia.org/wiki/URL_redirection?action=view#HTTP_status_codes_3xx`로 이동 (경로·쿼리·프래그먼트 유지) |
| 동시 생성 시 충돌·덮어쓰기 없음 | 50개 동시 요청 모두 성공, 번호 50개 모두 다름(82~131), 50개 모두 자기 원본으로 302 이동 |
| 재시작·재배포 후 유지 | Railway 재배포 전에 만든 `/54`가 두 번의 재배포 뒤에도 같은 원본으로 302 이동 |
| 잘못된 URL 거절 | 10종 모두 400 (`javascript:`, `ftp://`, 스킴 없음, `https:naver.com`, 빈 값, 공백, 호스트 없음, 2048자 초과, 공개 주소 자기 자신). Railway 주소로 된 단축 주소도 400 |
| 없는 번호 | `/999999999`, `/0`, `/01`, `/abc`, `/1/`, `/a/b` 모두 404 안내 페이지 |
| 수정 시도 거절 | `/api/links`의 PUT·PATCH·DELETE, `/숫자`의 POST·PUT·PATCH·DELETE 모두 405. 번호를 넣은 생성 요청은 새 번호를 받음. 시도 뒤에도 원래 주소로 이동 |

### 로컬 결과 (2026-11-08)

| 시나리오 | 결과 |
|---|---|
| 50개 동시 생성 | 50개 모두 성공, 번호 50개 모두 다름, 모두 자기 원본으로 302 이동 |
| 거절 규칙 | 25개 모두 통과 |
| 앱과 DB 재시작 | 재시작 전에 만든 `/58`이 재시작 뒤에도 같은 원본으로 이동. 다음 번호는 59로 이어짐 |
| 자동 테스트 | `cargo test` 34개 통과 (URL 검사, 번호 해석, DB 동시 저장·변경 거부, 이동·404) |

## 범위 밖

- 로그인, 수정·삭제, 만료, 통계는 만들지 않는다.
- 도배 방지(요청 횟수 제한)는 넣지 않았다.
- 원본 URL이 실제로 열리는 주소인지, 안전한 사이트인지는 확인하지 않는다.
