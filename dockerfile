FROM rust:alpine3.15 as builder
WORKDIR /app
RUN apk add musl-dev

# COPY ./.cargo .cargo
# COPY ./vendor vendor|
# Create empty dummy project to build deps
RUN cargo new code_harmony_api
RUN cd code_harmony_api
WORKDIR /app/code_harmony_api
COPY Cargo.toml Cargo.lock ./

# Get packages cached

RUN cargo build
RUN cargo clean -p code_harmony_api

#
COPY ./src src
RUN cargo install --path . --target=x86_64-unknown-linux-musl

FROM alpine:3.14
COPY --from=builder /usr/local/cargo/bin/main /usr/local/bin

ENV CH_HOST=0.0.0.0:8080
CMD /usr/local/bin/main

EXPOSE 8080