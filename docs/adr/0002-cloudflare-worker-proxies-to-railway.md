# Cloudflare Worker 하나가 공개 주소를 맡고, `/api`와 `/번호`는 Railway로 넘긴다

과제는 화면과 공개 입구를 Cloudflare에, 생성 API와 리다이렉트를 Railway의 Rust 서버에 두라고 한다. 그런데 단축 주소는 공개 주소 바로 아래(`/번호`)에서 동작해야 한다. 그래서 Cloudflare Worker 하나가 공개 주소(workers.dev)를 맡는다. 화면 파일은 Worker가 직접 주고, `/api/*`와 `/숫자` 요청은 Railway로 그대로 넘긴다. 브라우저 입장에서는 주소가 하나뿐이라 CORS 설정이 필요 없다. Railway 주소와 DB 접속 정보는 브라우저에 드러나지 않는다.

첫 화면 HTML도 Worker가 정적 파일로 준다. Dioxus fullstack은 서버가 그린 HTML 위에 WASM을 붙이므로(hydration), 배포 전에 같은 빌드의 서버를 잠깐 띄워 첫 화면 HTML을 받아 두고 화면 파일과 함께 올린다. 화면 파일 이름에는 빌드마다 다른 해시가 붙어서, Railway가 따로 빌드한 HTML과 섞어 쓸 수 없기 때문이다. 화면과 Railway 서버는 생성 API(`POST /api/links`, JSON)로만 이야기하므로, 두 빌드가 달라도 API가 같으면 함께 동작한다.

계획 단계에서는 이 방식이 안 되면 HTML은 Railway가 만들고 Worker는 정적 파일만 주는 방식으로 바꾸기로 했으나, 필요하지 않았다.

## Considered Options

- **Cloudflare Pages + 브라우저가 Railway API를 직접 호출**: 화면에서 생성은 되지만 `/번호` 리다이렉트를 Railway 주소에서만 할 수 있어, 공개 주소의 `/숫자`가 동작해야 한다는 조건을 못 지킨다. CORS 설정도 필요하다.
- **Cloudflare 없이 Railway만 사용**: 가장 단순하지만 과제의 배포 구성 조건을 어긴다.
