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
| `PUBLIC_ORIGIN` | 이 서비스의 공개 주소. 이 주소를 가리키는 URL은 단축을 거절한다. |

### 테스트

DB 테스트는 실제 PostgreSQL이 필요하다. 테스트마다 임시 DB를 새로 만들고 끝나면 지우므로, 개발용 데이터에는 영향이 없다.

```bash
docker compose up -d
set -a; . ./.env; set +a
cargo test --no-default-features --features server
```

> 나머지 내용(DB 준비, 환경 변수, 배포, 번호 발급 방식, 검증 결과)은 구현하면서 채운다.
