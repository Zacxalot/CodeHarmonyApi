FROM rust:alpine3.15 as builder
WORKDIR /app
RUN apk add musl-dev
RUN apk add openssl-dev

# COPY ./.cargo .cargo
# COPY ./vendor vendor|
# Create empty dummy project to build deps
RUN cargo new code_harmony_api
RUN cd code_harmony_api
WORKDIR /app/code_harmony_api
COPY Cargo.toml Cargo.lock ./

# Get packages cached

# RUN cargo build
# RUN cargo clean -p code_harmony_api

#
COPY ./src src
ARG RUSTFLAGS=-Ctarget-feature=-crt-static
RUN cargo install --path . --target=x86_64-unknown-linux-musl

FROM alpine:3.15
RUN apk add openssl libgcc
COPY --from=builder /usr/local/cargo/bin/code_harmony_api /usr/local/bin
COPY CA_CERT.pem ./

ENV CH_HOST=0.0.0.0:8080
CMD /usr/local/bin/code_harmony_api

EXPOSE 8080