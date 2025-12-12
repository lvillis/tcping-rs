# syntax=docker/dockerfile:1.5
FROM rust:1.92.0-alpine3.22 AS builder

RUN set -ex \
        \
    && apk update \
    && apk upgrade \
    && apk add --update --no-cache musl-dev openssl-dev perl make lld

WORKDIR /opt/app

COPY Cargo.toml /opt/app/Cargo.toml
COPY Cargo.lock /opt/app/Cargo.lock

RUN mkdir -p /opt/app/src && echo "fn main() {}" > /opt/app/src/main.rs

RUN --mount=type=cache,target=/usr/local/cargo/registry true \
    set -ex \
        \
    && cargo build --release

RUN rm -f /opt/app/src/main.rs
COPY src/ /opt/app/src/

RUN set -ex \
        \
    && export RUSTFLAGS="-C linker=lld" \
    && cargo build --release


FROM scratch AS runtime

COPY --from=builder /opt/app/target/release/tcping /usr/local/bin/tcping

ENTRYPOINT ["/usr/local/bin/tcping"]
