FROM rust AS builder
WORKDIR /build
COPY . .
RUN cargo build --release

FROM debian
COPY --from=builder /build/target/release/cloudflare-ddns /usr/bin/cloudflare-ddns
ENTRYPOINT ["cloudflare-ddns", "/config/cloudflare-ddns.yml"]
