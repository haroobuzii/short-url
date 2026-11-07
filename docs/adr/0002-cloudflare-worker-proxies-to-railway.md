# Cloudflare Worker 하나가 공개 주소를 맡고, `/api`와 `/번호`는 Railway로 넘긴다

과제는 화면과 공개 입구를 Cloudflare에, 생성 API와 리다이렉트를 Railway의 Rust 서버에 두라고 한다. 그런데 단축 주소는 공개 주소 바로 아래(`/번호`)에서 동작해야 한다. 그래서 Cloudflare Worker 하나가 공개 주소(workers.dev)를 맡는다. 화면 파일은 Worker가 직접 주고, `/api/*`와 `/숫자` 요청은 Railway로 그대로 넘긴다. 브라우저 입장에서는 주소가 하나뿐이라 CORS 설정이 필요 없다. Railway 주소와 DB 접속 정보는 브라우저에 드러나지 않는다.

화면 HTML을 어디서 만들지는 아직 확인이 필요하다. 먼저 화면 전체를 정적 파일로 만들어 Worker가 주는 방식을 시도한다. Dioxus fullstack에서 이 방식이 동작하지 않으면 HTML도 Railway가 만들고, Worker는 WASM·CSS·이미지 같은 정적 파일만 직접 주고 나머지는 넘기는 방식으로 바꾼다. 어느 쪽이든 사용자가 쓰는 공개 주소는 같다.

## Considered Options

- **Cloudflare Pages + 브라우저가 Railway API를 직접 호출**: 화면에서 생성은 되지만 `/번호` 리다이렉트를 Railway 주소에서만 할 수 있어, 공개 주소의 `/숫자`가 동작해야 한다는 조건을 못 지킨다. CORS 설정도 필요하다.
- **Cloudflare 없이 Railway만 사용**: 가장 단순하지만 과제의 배포 구성 조건을 어긴다.
