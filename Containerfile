FROM docker.io/library/rust:1.81-bookworm AS builder
WORKDIR /build
COPY . .
RUN cargo build --release

FROM docker.io/library/debian:bookworm
STOPSIGNAL SIGINT
RUN apt update && apt install -y ca-certificates && apt clean
COPY --from=builder /build/target/release/cloudflare-ddns /usr/bin/cloudflare-ddns
ENTRYPOINT ["cloudflare-ddns"]
