# Railway가 이 파일로 서버를 빌드하고 실행한다.
# Dioxus 공식 배포 문서(dioxuslabs.com/learn/0.7/tutorial/deploy)의 예시를 바탕으로 했다.

# 1) 빌드 도구가 들어 있는 기본 이미지
FROM rust:1-trixie AS chef
RUN cargo install cargo-chef --locked
WORKDIR /app

# 2) 의존성 목록만 뽑아 둔다. 내 코드만 바뀌었을 때 의존성을 다시 빌드하지 않게 하려는 것이다.
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# 3) 의존성을 먼저 빌드하고, 그다음 dx로 화면(WASM)과 서버를 함께 묶는다.
FROM chef AS builder
RUN rustup target add wasm32-unknown-unknown
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
RUN cargo binstall dioxus-cli@0.7.10 --root /.cargo -y --force
ENV PATH="/.cargo/bin:$PATH"
RUN dx bundle --web --release

# 4) 실행에 필요한 것만 담은 작은 이미지
FROM debian:trixie-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/dx/short-url/release/web/ /usr/local/app

# 모든 주소에서 접속을 받는다. Railway는 PORT를 따로 넣어 주며, 그 값이 이 기본값보다 우선한다.
ENV IP=0.0.0.0
ENV PORT=8080
EXPOSE 8080

WORKDIR /usr/local/app
ENTRYPOINT ["/usr/local/app/server"]
