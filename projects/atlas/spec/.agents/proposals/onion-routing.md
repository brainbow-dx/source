# Onion routing for WKWebView traffic

Status: raised 2026-08-18 as a one-off research question ("what would it take to support onion routing in our browser?"), answered informationally. No scope decision, no placement decision (Escher vs. Atlas), and no commitment to build has actually been made — this file exists only to record the technical answer already given, not as a scoped roadmap item.

## What routing would take (not decided, not started)

- **Routing**: embed `arti` (The Tor Project's official Rust reimplementation of Tor) in-process as a local SOCKS5 listener. Route WKWebView traffic through it via `WKWebsiteDataStore.proxyConfigurations` — the real API Apple added in macOS 14.4/iOS 17.4 for per-data-store SOCKS proxy configs, including DNS through the proxy so it doesn't leak to the system resolver. This piece looks feasible and reasonably scoped, if it's ever picked up.
- **Anonymity**: real Tor Browser anonymity depends on every user's browser looking identical (canvas/WebGL/font/timezone/screen-size normalization, disabled fingerprinting APIs). Without that, a plain WKWebView routed over Tor would actually be *more* fingerprintable on the network than a normal browser, not less — it would stand out as a unique client. This is a much larger, separate effort, called out here so it's never accidentally conflated with the routing piece above if this is picked up later.

## Open

Nothing built. Not yet scoped as a real roadmap item with milestones — that's the next step if/when picked up.
