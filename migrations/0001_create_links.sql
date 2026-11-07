-- 연결 하나 = 번호 하나 + 원본 URL 하나.
-- 번호는 DB가 매기고(GENERATED ALWAYS), 직접 지정할 수 없다.
-- 동시에 여러 요청이 와도 같은 번호가 나오지 않고, 한 번 나간 번호는 다시 나오지 않는다.
CREATE TABLE links (
    id           BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    original_url TEXT NOT NULL CHECK (char_length(original_url) BETWEEN 1 AND 2048),
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 연결은 만들어진 뒤 바뀌거나 사라지지 않는다.
-- 서버 코드에 실수가 있어도 DB가 수정·삭제를 거부하도록 한 번 더 막는다.
CREATE FUNCTION reject_link_change() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'links are immutable: % is not allowed', TG_OP;
END;
$$;

CREATE TRIGGER links_no_update_or_delete
    BEFORE UPDATE OR DELETE ON links
    FOR EACH ROW EXECUTE FUNCTION reject_link_change();

CREATE TRIGGER links_no_truncate
    BEFORE TRUNCATE ON links
    FOR EACH STATEMENT EXECUTE FUNCTION reject_link_change();
