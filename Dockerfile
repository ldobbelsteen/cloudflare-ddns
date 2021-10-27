FROM rust:alpine AS builder
RUN apk update && apk add --no-cache openssl-dev build-base
WORKDIR /build
COPY . .
RUN cargo build --release

FROM alpine
RUN apk update && apk add --no-cache ca-certificates
COPY --from=builder /build/target/release/cloudflare-ddns /usr/bin/cloudflare-ddns
CMD ["cloudflare-ddns", "/config/cloudflare.yml"]
