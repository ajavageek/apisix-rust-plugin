FROM rust:1.61.0

RUN rustup target add wasm32-wasi

RUN mkdir -p /var/wasm

ADD src src
ADD Cargo.toml .

RUN cargo build --target wasm32-wasi --release

RUN mv ./target/wasm32-wasi/release/sample.wasm /var/wasm/sample.wasm
