FROM rust:slim AS builder
WORKDIR /build
COPY . .
RUN cargo build --release

FROM debian:slim
RUN apt update && apt install -y ca-certificates
COPY --from=builder /build/target/release/cloudflare-ddns /usr/bin/cloudflare-ddns
ENTRYPOINT ["cloudflare-ddns"]
