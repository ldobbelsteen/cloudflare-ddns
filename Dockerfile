FROM rust AS builder
WORKDIR /build
COPY src src
COPY Cargo.* ./
RUN cargo build --release

FROM debian
RUN apt-get update && apt-get install -y ca-certificates
COPY --from=builder /build/target/release/cloudflare-ddns /usr/bin/cloudflare-ddns
CMD ["cloudflare-ddns", "/config/cloudflare.yml"]
