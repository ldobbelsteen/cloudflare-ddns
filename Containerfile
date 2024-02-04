FROM docker.io/library/rust:bookworm AS builder
WORKDIR /build
COPY . .
RUN cargo build --release

FROM docker.io/library/debian:bookworm
RUN apt update && apt install -y ca-certificates && apt clean
COPY --from=builder /build/target/release/cloudflare-ddns /usr/bin/cloudflare-ddns
ENTRYPOINT ["cloudflare-ddns"]
