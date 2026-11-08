// 서비스의 공개 입구. Cloudflare가 먼저 public/ 안의 화면 파일(HTML, JS, WASM, CSS)을 찾아서 주고,
// 화면 파일이 아닌 요청(/api/links, /번호, 없는 주소)만 이 코드로 들어온다.
// 이 코드는 그 요청을 그대로 Railway 백엔드로 넘기고, 응답도 그대로 돌려준다.

export default {
  async fetch(request, env) {
    const incoming = new URL(request.url);
    const target = new URL(incoming.pathname + incoming.search, env.BACKEND_ORIGIN);

    // 메서드·헤더·본문은 그대로 두고 주소만 Railway로 바꾼다.
    const proxied = new Request(target, request);

    // 302 이동 응답을 Worker가 따라가지 않고 브라우저에 그대로 전달해야
    // 브라우저 주소창이 원본 URL로 바뀐다.
    return fetch(proxied, { redirect: "manual" });
  },
};
