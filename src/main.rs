use dioxus::prelude::*;

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");

fn main() {
    dioxus::launch(App);
}

/// 서비스의 유일한 화면. 지금은 제목만 있고, 4단계에서 입력칸과 버튼을 붙인다.
#[component]
fn App() -> Element {
    rsx! {
        document::Title { "숫자형 단축 URL" }
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        main {
            h1 { "숫자형 단축 URL" }
            p { "긴 주소를 숫자로 된 짧은 주소로 바꿔 드립니다." }
        }
    }
}
