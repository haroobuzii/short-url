//! 단축 주소(`/번호`)로 들어온 요청을 원본 URL로 보내는 서버 처리.
//!
//! 화면을 거치지 않고 서버가 바로 302 응답으로 이동시킨다.
//! 번호가 아니거나 없는 번호면 404 안내 페이지를 돌려준다.

use dioxus::server::axum::{
    extract::Path,
    http::{header, HeaderValue, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use sqlx::PgPool;

/// 서버 라우터에 단축 주소 처리를 붙인다.
///
/// - `/번호`: 원본 URL로 이동하거나 404
/// - `/번호/`, `/a/b`처럼 한 칸보다 깊은 주소: 404
///
/// 화면 파일(`/assets/...`)과 API(`/api/links`)는 이름이 정해진 경로라서 여기보다 먼저 잡힌다.
pub fn add_routes(router: Router) -> Router {
    router
        .route("/{segment}", get(follow))
        .route("/{segment}/", get(not_found))
        .route("/{segment}/{*rest}", get(not_found))
}

async fn follow(Path(segment): Path<String>) -> Response {
    resolve(crate::server::pool(), &segment).await
}

async fn not_found() -> Response {
    not_found_page()
}

/// 주소의 한 칸을 번호로 읽는다. 1 이상의 정수를 0 없이 쓴 것만 번호로 인정한다.
pub fn parse_number(segment: &str) -> Option<i64> {
    let all_digits = !segment.is_empty() && segment.bytes().all(|b| b.is_ascii_digit());
    if !all_digits || segment.starts_with('0') {
        return None;
    }
    // i64 범위를 넘으면 parse가 실패해서 None이 된다.
    segment.parse().ok()
}

/// 번호를 찾아 이동 응답이나 404 응답을 만든다.
pub async fn resolve(pool: &PgPool, segment: &str) -> Response {
    let Some(id) = parse_number(segment) else {
        return not_found_page();
    };

    match crate::db::find_link(pool, id).await {
        Ok(Some(original_url)) => redirect_to(&original_url),
        Ok(None) => not_found_page(),
        Err(e) => {
            dioxus::logger::tracing::error!("연결 조회 실패: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, Html(ERROR_HTML)).into_response()
        }
    }
}

/// 302 응답을 만든다.
///
/// HTTP 헤더에는 영어·숫자·기호만 넣을 수 있다. 그래서 한글 같은 글자는 브라우저가
/// 주소창에 입력받았을 때와 똑같은 규칙(WHATWG URL)으로 `%EB%8C%80`처럼 바꿔서 보낸다.
/// 경로·쿼리·프래그먼트의 뜻은 그대로다.
fn redirect_to(original_url: &str) -> Response {
    let location = url::Url::parse(original_url)
        .map(String::from)
        .unwrap_or_else(|_| original_url.to_string());

    match HeaderValue::from_str(&location) {
        Ok(value) => (StatusCode::FOUND, [(header::LOCATION, value)]).into_response(),
        Err(_) => {
            dioxus::logger::tracing::error!("Location 헤더로 쓸 수 없는 주소: {location}");
            (StatusCode::INTERNAL_SERVER_ERROR, Html(ERROR_HTML)).into_response()
        }
    }
}

fn not_found_page() -> Response {
    (StatusCode::NOT_FOUND, Html(NOT_FOUND_HTML)).into_response()
}

const NOT_FOUND_HTML: &str = r#"<!doctype html>
<html lang="ko">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>없는 단축 주소 - 숫자형 단축 URL</title>
<style>
  body { margin: 0; font-family: -apple-system, BlinkMacSystemFont, "Apple SD Gothic Neo", "Malgun Gothic", sans-serif; background: #f7f7f8; color: #1a1a1a; }
  main { max-width: 560px; margin: 80px auto; padding: 0 16px; }
  a { display: inline-block; margin-top: 16px; padding: 12px 18px; border-radius: 8px; background: #1f6feb; color: white; text-decoration: none; }
</style>
</head>
<body>
<main>
  <h1>존재하지 않는 단축 주소입니다</h1>
  <p>주소를 잘못 입력했거나, 만들어진 적 없는 번호예요.</p>
  <a href="/">새 단축 주소 만들기</a>
</main>
</body>
</html>
"#;

const ERROR_HTML: &str = r#"<!doctype html>
<html lang="ko">
<head><meta charset="utf-8"><title>오류 - 숫자형 단축 URL</title></head>
<body><p>잠시 문제가 생겼어요. 조금 뒤 다시 시도해 주세요.</p></body>
</html>
"#;

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::*;
    use crate::db::create_link;

    #[test]
    fn 평범한_숫자는_번호다() {
        assert_eq!(parse_number("1"), Some(1));
        assert_eq!(parse_number("123"), Some(123));
        assert_eq!(parse_number("9223372036854775807"), Some(i64::MAX));
    }

    #[test]
    fn 영과_앞에_붙은_영은_번호가_아니다() {
        assert_eq!(parse_number("0"), None);
        assert_eq!(parse_number("0123"), None);
        assert_eq!(parse_number("00"), None);
    }

    #[test]
    fn 숫자가_아닌_글자가_섞이면_번호가_아니다() {
        for segment in ["", "abc", "1a", "-1", "+1", "1.0", " 1", "１", "٣"] {
            assert_eq!(parse_number(segment), None, "{segment:?}");
        }
    }

    #[test]
    fn 너무_큰_수는_번호가_아니다() {
        assert_eq!(parse_number("9223372036854775808"), None);
        assert_eq!(parse_number(&"9".repeat(100)), None);
    }

    fn location(response: &Response) -> &str {
        response.headers()[header::LOCATION].to_str().unwrap()
    }

    #[sqlx::test]
    async fn 있는_번호는_302로_원본에_보낸다(pool: PgPool) {
        let id = create_link(&pool, "https://example.org/path/to?q=1&lang=ko#section-2")
            .await
            .unwrap();
        let response = resolve(&pool, &id.to_string()).await;
        assert_eq!(response.status(), StatusCode::FOUND);
        assert_eq!(location(&response), "https://example.org/path/to?q=1&lang=ko#section-2");
    }

    #[sqlx::test]
    async fn 한글_주소는_브라우저가_쓰는_인코딩으로_보낸다(pool: PgPool) {
        let id = create_link(&pool, "https://ko.wikipedia.org/wiki/대한민국?q=역사#개요")
            .await
            .unwrap();
        let response = resolve(&pool, &id.to_string()).await;
        assert_eq!(response.status(), StatusCode::FOUND);
        assert_eq!(
            location(&response),
            "https://ko.wikipedia.org/wiki/%EB%8C%80%ED%95%9C%EB%AF%BC%EA%B5%AD?q=%EC%97%AD%EC%82%AC#%EA%B0%9C%EC%9A%94"
        );
    }

    #[sqlx::test]
    async fn 없는_번호는_404다(pool: PgPool) {
        let response = resolve(&pool, "999").await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert!(response.headers().get(header::LOCATION).is_none());
    }

    #[sqlx::test]
    async fn 번호가_아니면_DB를_보지_않고_404다(pool: PgPool) {
        let id = create_link(&pool, "https://example.org").await.unwrap();
        assert_eq!(id, 1);
        for segment in ["01", "abc", "0"] {
            let response = resolve(&pool, segment).await;
            assert_eq!(response.status(), StatusCode::NOT_FOUND, "{segment:?}");
        }
    }

    #[sqlx::test]
    async fn 없는_번호_페이지는_한국어_안내와_새로_만들기_링크가_있다(pool: PgPool) {
        let response = resolve(&pool, "999").await;
        assert_eq!(
            response.headers()[header::CONTENT_TYPE],
            "text/html; charset=utf-8"
        );
        let body = dioxus::server::axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let html = String::from_utf8(body.to_vec()).unwrap();
        assert!(html.contains("존재하지 않는 단축 주소"));
        assert!(html.contains(r#"href="/""#));
    }
}
