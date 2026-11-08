# 숫자형 단축 URL 서비스

## 배포 URL

- 서비스 URL: (배포 후 작성)
- 테스트 단축 URL: (배포 후 작성)
- 테스트 원본 URL: (배포 후 작성)

## 로컬 실행

1. 환경 변수 파일을 만든다. 들어 있는 값은 로컬 개발용이다.

   ```bash
   cp .env.example .env
   ```

2. Docker로 PostgreSQL을 띄운다.

   ```bash
   docker compose up -d
   ```

3. 앱을 실행하고 브라우저에서 http://localhost:8080 을 연다.

   ```bash
   dx serve --platform web
   ```

### 환경 변수

| 이름 | 뜻 |
|---|---|
| `DATABASE_URL` | PostgreSQL 접속 주소 |
| `PUBLIC_ORIGIN` | 이 서비스의 공개 주소. 이 주소를 가리키는 URL은 단축을 거절한다. 여러 개면 쉼표로 구분한다. |

### 테스트

DB 테스트는 실제 PostgreSQL이 필요하다. 테스트마다 임시 DB를 새로 만들고 끝나면 지우므로, 개발용 데이터에는 영향이 없다.

```bash
docker compose up -d
set -a; . ./.env; set +a
cargo test --no-default-features --features server
```

## 검증 시나리오

`scripts/`의 스크립트는 로컬 주소와 배포 주소 모두에 쓸 수 있다.

```bash
scripts/check-concurrency.sh http://localhost:8080 50   # 동시 생성
scripts/check-rules.sh http://localhost:8080            # 잘못된 URL, 없는 번호, 수정 시도 거절
scripts/check-persistence.sh http://localhost:8080 create
# 서버와 DB를 재시작한 뒤
scripts/check-persistence.sh http://localhost:8080 verify
```

### 로컬 결과 (2026-11-08)

| 시나리오 | 결과 |
|---|---|
| 50개 동시 생성 | 50개 모두 성공, 번호 50개 모두 다름(6~55), 50개 모두 자기 원본으로 302 이동 |
| 잘못된 URL 10종 | 모두 400 (`javascript:`, `ftp://`, 스킴 없음, `https:naver.com`, 빈 값, 공백, 호스트 없음, 2048자 초과, 자기 주소) |
| 없는 번호·번호가 아닌 주소 6종 | 모두 404 (`/999999999`, `/0`, `/01`, `/abc`, `/1/`, `/a/b`) |
| 수정·삭제 시도 | `/api/links`의 PUT·PATCH·DELETE, `/번호`의 POST·PUT·PATCH·DELETE 모두 405. 번호를 지정한 생성 요청은 무시되고 새 번호를 받음. 시도 뒤에도 원래 주소로 이동 |
| 앱과 DB 재시작 | 재시작 전에 만든 `/58`이 재시작 뒤에도 같은 원본으로 302 이동. 다음 번호는 59로 이어짐 |

### Railway 백엔드 결과 (2026-11-08, https://short-url-production-da76.up.railway.app)

| 시나리오 | 결과 |
|---|---|
| 50개 동시 생성 | 50개 모두 성공, 번호 50개 모두 다름(1~50), 50개 모두 자기 원본으로 302 이동 |
| 거절 규칙 (`check-rules.sh`) | 25개 모두 통과 (잘못된 URL 10종 400, 없는 번호 6종 404, 수정·삭제 시도 9종 거절) |
| 재배포 | 재배포 전에 만든 `/54`가 재배포 뒤에도 같은 원본으로 302 이동 |

> Cloudflare 결과, Railway·Cloudflare 배포 방법, 번호 발급 방식 설명은 다음 단계에서 채운다.
