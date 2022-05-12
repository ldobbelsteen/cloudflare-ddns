FROM rust AS builder
WORKDIR /build
COPY . .
RUN cargo build --release

FROM debian
RUN apt update && apt install -y ca-certificates
COPY --from=builder /build/target/release/cloudflare-ddns /usr/bin/cloudflare-ddns
ENTRYPOINT ["cloudflare-ddns", "/config/cloudflare-ddns.yml"]
