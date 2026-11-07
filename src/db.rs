//! 연결을 DB에 저장하고 꺼내는 코드. 서버에서만 쓴다.
//!
//! 연결은 만들기와 조회만 있고, 수정·삭제 함수는 일부러 두지 않는다.

use sqlx::postgres::{PgPool, PgPoolOptions};

/// DB에 접속하고, 아직 적용하지 않은 마이그레이션(표 만들기 등)을 적용한다.
pub async fn connect(database_url: &str) -> Result<PgPool, Box<dyn std::error::Error + Send + Sync>> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await?;
    sqlx::migrate!().run(&pool).await?;
    Ok(pool)
}

/// 새 연결을 만들고 DB가 매긴 번호를 돌려준다.
///
/// `original_url`은 `validate` 규칙을 이미 통과한 값이어야 한다.
pub async fn create_link(pool: &PgPool, original_url: &str) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar("INSERT INTO links (original_url) VALUES ($1) RETURNING id")
        .bind(original_url)
        .fetch_one(pool)
        .await
}

/// 번호로 원본 URL을 찾는다. 없는 번호면 `None`.
pub async fn find_link(pool: &PgPool, id: i64) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT original_url FROM links WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

/// 이 테스트들은 실제 PostgreSQL이 필요하다.
/// `docker compose up -d`로 DB를 띄우고 `DATABASE_URL`을 설정한 뒤 실행한다.
/// `#[sqlx::test]`는 테스트마다 빈 임시 DB를 만들고 마이그레이션을 적용한 뒤, 끝나면 지운다.
#[cfg(test)]
#[allow(non_snake_case)] // 테스트 이름에 URL, DB, None 같은 대문자 단어를 그대로 쓴다.
mod tests {
    use super::*;

    #[sqlx::test]
    async fn 번호는_1부터_차례로_매긴다(pool: PgPool) {
        assert_eq!(create_link(&pool, "https://a.example").await.unwrap(), 1);
        assert_eq!(create_link(&pool, "https://b.example").await.unwrap(), 2);
    }

    #[sqlx::test]
    async fn 같은_원본_URL도_매번_새_번호를_받는다(pool: PgPool) {
        let first = create_link(&pool, "https://naver.com").await.unwrap();
        let second = create_link(&pool, "https://naver.com").await.unwrap();
        assert_ne!(first, second);
    }

    #[sqlx::test]
    async fn 저장한_원본_URL을_그대로_돌려준다(pool: PgPool) {
        let url = "https://ko.wikipedia.org/wiki/대한민국?q=1#역사";
        let id = create_link(&pool, url).await.unwrap();
        assert_eq!(find_link(&pool, id).await.unwrap().as_deref(), Some(url));
    }

    #[sqlx::test]
    async fn 없는_번호는_None이다(pool: PgPool) {
        assert_eq!(find_link(&pool, 999).await.unwrap(), None);
    }

    #[sqlx::test]
    async fn DB가_수정을_거부한다(pool: PgPool) {
        let id = create_link(&pool, "https://a.example").await.unwrap();
        let result = sqlx::query("UPDATE links SET original_url = 'https://evil.example' WHERE id = $1")
            .bind(id)
            .execute(&pool)
            .await;
        assert!(result.is_err());
        assert_eq!(find_link(&pool, id).await.unwrap().as_deref(), Some("https://a.example"));
    }

    #[sqlx::test]
    async fn DB가_삭제를_거부한다(pool: PgPool) {
        let id = create_link(&pool, "https://a.example").await.unwrap();
        let delete = sqlx::query("DELETE FROM links WHERE id = $1").bind(id).execute(&pool).await;
        assert!(delete.is_err());
        let truncate = sqlx::query("TRUNCATE links").execute(&pool).await;
        assert!(truncate.is_err());
        assert!(find_link(&pool, id).await.unwrap().is_some());
    }

    #[sqlx::test]
    async fn 번호를_직접_지정해서_넣을_수_없다(pool: PgPool) {
        let result = sqlx::query("INSERT INTO links (id, original_url) VALUES (7, 'https://a.example')")
            .execute(&pool)
            .await;
        assert!(result.is_err());
    }

    #[sqlx::test]
    async fn DB도_2048자를_넘는_URL을_거부한다(pool: PgPool) {
        let too_long = format!("https://a.example/{}", "a".repeat(2048));
        assert!(create_link(&pool, &too_long).await.is_err());
    }

    #[sqlx::test]
    async fn 저장에_실패한_번호는_다시_쓰지_않는다(pool: PgPool) {
        let first = create_link(&pool, "https://a.example").await.unwrap();
        let _ = create_link(&pool, "").await; // 빈 값이라 DB가 거부한다.
        let next = create_link(&pool, "https://b.example").await.unwrap();
        // 실패한 저장도 번호 하나를 쓰므로, 번호 사이에 빈칸이 생긴다.
        assert_eq!(next, first + 2);
    }
}
