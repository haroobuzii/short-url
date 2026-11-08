//! 원본 URL을 받아 줄지 거절할지 정하는 규칙.
//!
//! 화면이 아니라 서버가 이 규칙으로 최종 판단한다.

/// 원본 URL이 거절된 이유. 각 이유는 사용자에게 보여 줄 한국어 문구를 가진다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvalidUrl {
    Empty,
    TooLong,
    NotHttp,
    HasSpace,
    Malformed,
    PointsToUs,
}

impl InvalidUrl {
    pub fn message(&self) -> &'static str {
        match self {
            InvalidUrl::Empty => "주소를 입력해 주세요.",
            InvalidUrl::TooLong => "주소가 너무 깁니다. 2048자 이하로 입력해 주세요.",
            InvalidUrl::NotHttp => "http:// 또는 https://로 시작하는 주소만 줄일 수 있어요.",
            InvalidUrl::HasSpace => "주소 중간에 공백이나 줄바꿈이 있으면 안 됩니다.",
            InvalidUrl::Malformed => "올바른 주소 형식이 아닙니다.",
            InvalidUrl::PointsToUs => "이 서비스의 단축 주소는 다시 줄일 수 없어요.",
        }
    }
}

/// 원본 URL의 최대 길이(글자 수).
pub const MAX_LEN: usize = 2048;

/// 입력을 검사하고, 통과하면 저장할 원본 URL(앞뒤 공백만 뺀 입력 그대로)을 돌려준다.
///
/// `our_hosts`는 이 서비스에 들어오는 공개 호스트 이름들(예: Cloudflare와 Railway 주소)이다.
/// 그중 하나를 가리키는 주소는 거절한다. 비어 있으면 이 검사를 건너뛴다.
pub fn validate_original_url(input: &str, our_hosts: &[&str]) -> Result<String, InvalidUrl> {
    let url = input.trim();

    if url.is_empty() {
        return Err(InvalidUrl::Empty);
    }
    if url.chars().count() > MAX_LEN {
        return Err(InvalidUrl::TooLong);
    }
    if url.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(InvalidUrl::HasSpace);
    }

    // `http://` 또는 `https://`로 정확히 시작해야 한다. 대소문자는 가리지 않는다.
    // `https:naver.com`처럼 슬래시가 빠진 주소는 브라우저가 다르게 해석할 수 있어서 막는다.
    let lower = url.to_ascii_lowercase();
    let after_scheme = if lower.starts_with("http://") {
        &url["http://".len()..]
    } else if lower.starts_with("https://") {
        &url["https://".len()..]
    } else {
        return Err(InvalidUrl::NotHttp);
    };

    // `http:///path`처럼 슬래시가 더 붙으면 파서가 `path`를 호스트로 착각한다.
    if after_scheme.starts_with(['/', '\\']) {
        return Err(InvalidUrl::Malformed);
    }

    let parsed = url::Url::parse(url).map_err(|_| InvalidUrl::Malformed)?;
    let host = match parsed.host_str() {
        Some(host) if !host.is_empty() => host,
        _ => return Err(InvalidUrl::Malformed),
    };

    if our_hosts.iter().any(|ours| host.eq_ignore_ascii_case(ours)) {
        return Err(InvalidUrl::PointsToUs);
    }

    Ok(url.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const OUR_HOSTS: &[&str] = &["short.example", "short-url.up.railway.app"];

    fn ok(input: &str) -> String {
        validate_original_url(input, OUR_HOSTS).expect("통과해야 하는 주소")
    }

    fn err(input: &str) -> InvalidUrl {
        validate_original_url(input, OUR_HOSTS).expect_err("거절해야 하는 주소")
    }

    #[test]
    fn 평범한_http와_https는_통과한다() {
        assert_eq!(ok("https://naver.com"), "https://naver.com");
        assert_eq!(ok("http://example.org/a"), "http://example.org/a");
    }

    #[test]
    fn 경로_쿼리_프래그먼트를_그대로_보관한다() {
        let url = "https://example.org/path/to?q=1&lang=ko#section-2";
        assert_eq!(ok(url), url);
    }

    #[test]
    fn 앞뒤_공백만_지운다() {
        assert_eq!(ok("  https://naver.com \n"), "https://naver.com");
    }

    #[test]
    fn 대문자_스킴도_통과한다() {
        assert_eq!(ok("HTTPS://naver.com"), "HTTPS://naver.com");
    }

    #[test]
    fn 한글이_들어간_주소도_통과한다() {
        let url = "https://ko.wikipedia.org/wiki/대한민국";
        assert_eq!(ok(url), url);
    }

    #[test]
    fn 빈_입력은_거절한다() {
        assert_eq!(err(""), InvalidUrl::Empty);
        assert_eq!(err("   "), InvalidUrl::Empty);
    }

    #[test]
    fn 길이는_2048자까지만_받는다() {
        let base = "https://example.org/";
        let at_limit = format!("{base}{}", "a".repeat(MAX_LEN - base.len()));
        assert_eq!(ok(&at_limit), at_limit);
        let over = format!("{at_limit}a");
        assert_eq!(err(&over), InvalidUrl::TooLong);
    }

    #[test]
    fn http와_https가_아닌_스킴은_거절한다() {
        assert_eq!(err("javascript:alert(1)"), InvalidUrl::NotHttp);
        assert_eq!(err("ftp://example.org"), InvalidUrl::NotHttp);
        assert_eq!(err("data:text/html,hi"), InvalidUrl::NotHttp);
        assert_eq!(err("naver.com"), InvalidUrl::NotHttp);
    }

    #[test]
    fn 슬래시_두개가_빠진_주소는_거절한다() {
        // 브라우저가 `https:naver.com`을 우리 사이트 안의 경로로 해석할 수 있다.
        assert_eq!(err("https:naver.com"), InvalidUrl::NotHttp);
        assert_eq!(err("https:/naver.com"), InvalidUrl::NotHttp);
        assert_eq!(err("https:\\\\naver.com"), InvalidUrl::NotHttp);
    }

    #[test]
    fn 중간에_공백이나_줄바꿈이_있으면_거절한다() {
        assert_eq!(err("https://naver.com/a b"), InvalidUrl::HasSpace);
        assert_eq!(err("https://naver.com/a\nb"), InvalidUrl::HasSpace);
        assert_eq!(err("https://naver.com/a\tb"), InvalidUrl::HasSpace);
    }

    #[test]
    fn 호스트가_없거나_형식이_틀리면_거절한다() {
        assert_eq!(err("https://"), InvalidUrl::Malformed);
        assert_eq!(err("http:///path"), InvalidUrl::Malformed);
        assert_eq!(err("https://exa mple.com"), InvalidUrl::HasSpace);
        assert_eq!(err("https://[::1"), InvalidUrl::Malformed);
    }

    #[test]
    fn 우리_서비스_주소는_거절한다() {
        assert_eq!(err("https://short.example/7"), InvalidUrl::PointsToUs);
        assert_eq!(err("http://SHORT.example"), InvalidUrl::PointsToUs);
        assert_eq!(err("https://short.example:443/1"), InvalidUrl::PointsToUs);
    }

    #[test]
    fn 우리_주소가_여러_개면_모두_거절한다() {
        assert_eq!(err("https://short-url.up.railway.app/7"), InvalidUrl::PointsToUs);
    }

    #[test]
    fn 비슷하지만_다른_호스트는_통과한다() {
        assert_eq!(ok("https://notshort.example/7"), "https://notshort.example/7");
        assert_eq!(ok("https://sub.short.example/7"), "https://sub.short.example/7");
    }

    #[test]
    fn 우리_주소를_모르면_그_규칙은_건너뛴다() {
        let url = "https://short.example/7";
        assert_eq!(validate_original_url(url, &[]), Ok(url.to_string()));
    }
}
