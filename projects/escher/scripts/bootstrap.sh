#!/usr/bin/env bash

ESCHER_BIN="$HOME/.escher/bin"

mkdir -p "$ESCHER_BIN"

# TODO: Add --with-docker to build caddy (etc) with a docker builder container.

which xcaddy
which caddy

# TODO: Add 
xcaddy build \
    --with github.com/tailscale/caddy-tailscale \
    --with github.com/caddy-dns/cloudflare \
    --output "$ESCHER_BIN/caddy"
