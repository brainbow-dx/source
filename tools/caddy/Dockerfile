FROM golang:1.26.5-bookworm AS builder

RUN go install github.com/caddyserver/xcaddy/cmd/xcaddy@v0.4.5

WORKDIR /build

ARG TARGETOS=linux
ARG TARGETARCH=amd64
ENV GOOS=$TARGETOS
ENV GOARCH=$TARGETARCH
ENV CGO_ENABLED=0

RUN set -eux; \
    EXT=""; \
    if [ "$TARGETOS" = "windows" ]; then EXT=".exe"; fi; \
    xcaddy build v2.11.4 \
      --with github.com/tailscale/caddy-tailscale \
      --with github.com/lindenlab/caddy-s3-proxy@v0.5.6 \
      --output /build/caddy${EXT}

FROM scratch AS export
COPY --from=builder /build/caddy* /