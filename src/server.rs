//! 서버가 켜질 때 한 번 준비하는 것들: 환경 변수 읽기, DB 접속, 라우터 만들기.

use std::sync::OnceLock;

use anyhow::{anyhow, Context};
use dioxus::server::axum::Router;
use sqlx::PgPool;

static POOL: OnceLock<PgPool> = OnceLock::new();
static OUR_HOSTS: OnceLock<Vec<&'static str>> = OnceLock::new();

/// 서버 라우터를 만든다. 처음 불릴 때 환경 변수를 읽고 DB에 접속한다.
///
/// 개발 중에는 코드를 고칠 때마다 다시 불리므로, DB 접속은 한 번만 한다.
pub async fn router() -> anyhow::Result<Router> {
    if POOL.get().is_none() {
        // 로컬에서는 .env 파일에서, Railway에서는 서비스 설정에서 환경 변수를 받는다.
        let _ = dotenvy::dotenv();

        let our_hosts = read_our_hosts()?;
        let _ = OUR_HOSTS.set(our_hosts);

        let database_url =
            std::env::var("DATABASE_URL").context("DATABASE_URL 환경 변수가 필요합니다")?;
        let pool = crate::db::connect(&database_url)
            .await
            .map_err(|e| anyhow!("DB에 접속하지 못했습니다: {e}"))?;
        let _ = POOL.set(pool);
    }

    Ok(crate::redirect::add_routes(dioxus::server::router(crate::App)))
}

pub fn pool() -> &'static PgPool {
    POOL.get().expect("router()가 먼저 DB에 접속해야 합니다")
}

/// 이 서비스에 들어오는 공개 호스트 이름들. `PUBLIC_ORIGIN`이 비어 있으면 빈 목록.
pub fn our_hosts() -> &'static [&'static str] {
    OUR_HOSTS.get().map(Vec::as_slice).unwrap_or(&[])
}

/// `PUBLIC_ORIGIN`을 읽는다. 쉼표로 여러 주소를 적을 수 있다.
/// 예: `https://short-url.example.workers.dev,https://short-url-production.up.railway.app`
fn read_our_hosts() -> anyhow::Result<Vec<&'static str>> {
    let value = std::env::var("PUBLIC_ORIGIN").unwrap_or_default();
    let mut hosts = Vec::new();
    for origin in value.split(',').map(str::trim).filter(|o| !o.is_empty()) {
        let parsed = url::Url::parse(origin)
            .with_context(|| format!("PUBLIC_ORIGIN이 올바른 주소가 아닙니다: {origin}"))?;
        let host = parsed
            .host_str()
            .ok_or_else(|| anyhow!("PUBLIC_ORIGIN에 호스트가 없습니다: {origin}"))?;
        // 서버가 켜져 있는 동안 계속 쓰는 값이라 한 번만 만들어 두고 놓지 않는다.
        hosts.push(&*Box::leak(host.to_string().into_boxed_str()));
    }
    Ok(hosts)
}
