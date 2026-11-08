use dioxus::prelude::*;

#[cfg(feature = "server")]
mod db;
#[cfg(feature = "server")]
mod redirect;
#[cfg(feature = "server")]
mod server;
#[cfg(feature = "server")]
mod validate;

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");

fn main() {
    // 서버로 빌드할 때: DB에 접속한 뒤 화면과 API를 함께 제공한다.
    #[cfg(feature = "server")]
    dioxus::serve(|| async move { server::router().await });

    // 브라우저(WASM)로 빌드할 때: 화면만 띄운다.
    #[cfg(not(feature = "server"))]
    dioxus::launch(App);
}

/// 원본 URL을 받아 새 연결을 만들고 번호를 돌려준다.
///
/// 검사는 반드시 여기(서버)에서 한다. 거절하면 400과 한국어 이유를 돌려준다.
#[post("/api/links")]
async fn shorten(url: String) -> Result<i64, ServerFnError> {
    let original_url = match validate::validate_original_url(&url, server::our_hosts()) {
        Ok(original_url) => original_url,
        Err(reason) => return Ok(HttpError::bad_request(reason.message())?),
    };

    match db::create_link(server::pool(), &original_url).await {
        Ok(id) => Ok(id),
        Err(e) => {
            dioxus::logger::tracing::error!("연결 저장 실패: {e}");
            Ok(HttpError::internal_server_error(
                "저장하지 못했어요. 잠시 뒤 다시 시도해 주세요.",
            )?)
        }
    }
}

/// 서비스의 유일한 화면.
#[component]
fn App() -> Element {
    let mut input = use_signal(String::new);
    let mut pending = use_signal(|| false);
    let mut short_url = use_signal(|| None::<String>);
    let mut error = use_signal(|| None::<String>);
    let mut copied = use_signal(|| false);

    let submit = move |event: FormEvent| {
        // 폼 제출 때 페이지가 새로 고쳐지지 않게 한다.
        event.prevent_default();
        if pending() {
            return;
        }
        spawn(async move {
            pending.set(true);
            error.set(None);
            copied.set(false);

            match shorten(input()).await {
                Ok(id) => {
                    // 서버는 번호만 준다. 지금 보고 있는 사이트 주소 뒤에 붙여 단축 주소를 만든다.
                    let origin = document::eval("return window.location.origin")
                        .join::<String>()
                        .await
                        .unwrap_or_default();
                    short_url.set(Some(format!("{origin}/{id}")));
                }
                Err(e) => {
                    short_url.set(None);
                    error.set(Some(error_message(e)));
                }
            }

            pending.set(false);
        });
    };

    let copy = move |_| async move {
        let Some(text) = short_url() else { return };
        let script = document::eval(COPY_SCRIPT);
        let sent = script.send(text);
        if sent.is_ok() && script.join::<bool>().await.is_ok() {
            error.set(None);
            copied.set(true);
        } else {
            error.set(Some("복사하지 못했어요. 주소를 직접 선택해서 복사해 주세요.".into()));
        }
    };

    rsx! {
        document::Title { "숫자형 단축 URL" }
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        main {
            h1 { "숫자형 단축 URL" }
            p { class: "lead", "긴 주소를 숫자로 된 짧은 주소로 바꿔 드립니다." }

            form { class: "shorten", onsubmit: submit,
                input {
                    r#type: "text",
                    name: "url",
                    placeholder: "https://example.org/아주/긴/주소",
                    autocomplete: "off",
                    value: "{input}",
                    oninput: move |event| input.set(event.value()),
                }
                button { r#type: "submit", disabled: pending(),
                    if pending() { "줄이는 중…" } else { "줄이기" }
                }
            }

            if let Some(message) = error() {
                p { class: "error", role: "alert", "{message}" }
            }

            if let Some(url) = short_url() {
                div { class: "result",
                    a { href: "{url}", target: "_blank", rel: "noopener", "{url}" }
                    button { r#type: "button", onclick: copy,
                        if copied() { "복사됨" } else { "복사" }
                    }
                }
            }
        }
    }
}

/// 받은 글자를 클립보드에 복사한다. 최신 클립보드 API가 막힌 브라우저에서는
/// 숨긴 입력칸을 선택해 복사하는 예전 방식으로 한 번 더 시도한다.
const COPY_SCRIPT: &str = r#"
const text = await dioxus.recv();
try {
    await navigator.clipboard.writeText(text);
    return true;
} catch (_) {
    const area = document.createElement("textarea");
    area.value = text;
    area.setAttribute("readonly", "");
    area.style.position = "fixed";
    area.style.opacity = "0";
    document.body.appendChild(area);
    area.select();
    const ok = document.execCommand("copy");
    area.remove();
    if (!ok) throw new Error("copy failed");
    return true;
}
"#;

/// 서버 함수 에러를 사용자에게 보여 줄 문구로 바꾼다.
fn error_message(error: ServerFnError) -> String {
    match error {
        ServerFnError::ServerError { message, .. } => message,
        _ => "서버에 연결하지 못했어요. 잠시 뒤 다시 시도해 주세요.".into(),
    }
}
