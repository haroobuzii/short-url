//! 서버가 켜질 때 한 번 준비하는 것들: 환경 변수 읽기, DB 접속, 라우터 만들기.

use std::sync::OnceLock;

use anyhow::{anyhow, Context};
use dioxus::server::axum::Router;
use sqlx::PgPool;

static POOL: OnceLock<PgPool> = OnceLock::new();
static OUR_HOST: OnceLock<Option<String>> = OnceLock::new();

/// 서버 라우터를 만든다. 처음 불릴 때 환경 변수를 읽고 DB에 접속한다.
///
/// 개발 중에는 코드를 고칠 때마다 다시 불리므로, DB 접속은 한 번만 한다.
pub async fn router() -> anyhow::Result<Router> {
    if POOL.get().is_none() {
        // 로컬에서는 .env 파일에서, Railway에서는 서비스 설정에서 환경 변수를 받는다.
        let _ = dotenvy::dotenv();

        let our_host = read_our_host()?;
        let _ = OUR_HOST.set(our_host);

        let database_url =
            std::env::var("DATABASE_URL").context("DATABASE_URL 환경 변수가 필요합니다")?;
        let pool = crate::db::connect(&database_url)
            .await
            .map_err(|e| anyhow!("DB에 접속하지 못했습니다: {e}"))?;
        let _ = POOL.set(pool);
    }

    Ok(dioxus::server::router(crate::App))
}

pub fn pool() -> &'static PgPool {
    POOL.get().expect("router()가 먼저 DB에 접속해야 합니다")
}

/// 이 서비스의 공개 호스트 이름. `PUBLIC_ORIGIN`이 비어 있으면 `None`.
pub fn our_host() -> Option<&'static str> {
    OUR_HOST.get().and_then(|host| host.as_deref())
}

fn read_our_host() -> anyhow::Result<Option<String>> {
    let origin = std::env::var("PUBLIC_ORIGIN").unwrap_or_default();
    let origin = origin.trim();
    if origin.is_empty() {
        return Ok(None);
    }
    let parsed = url::Url::parse(origin)
        .with_context(|| format!("PUBLIC_ORIGIN이 올바른 주소가 아닙니다: {origin}"))?;
    let host = parsed
        .host_str()
        .ok_or_else(|| anyhow!("PUBLIC_ORIGIN에 호스트가 없습니다: {origin}"))?;
    Ok(Some(host.to_string()))
}
