# AGENTS.md

Guidance for AI agents working with this repository.

## Overview
Janus is a stateless RESTful API gateway that posts to Bilibili dynamics, receives Aliyun OSS EventBridge webhooks, and refreshes Aliyun CDN. No database - pure HTTP API service.

## Tech Stack
- **Axum 0.8** + Tower middleware
- **Reqwest** for external APIs
- **Utoipa** for OpenAPI (Scalar UI at `/api/scalar`)
- **ES256 JWT** (ECDSA P-256) for Bilibili auth
- **Aliyun V3 signature** for OSS EventBridge
- **mimalloc** global allocator

## Commands
```bash
cargo build
cargo run -- server --config config.toml
cargo run -- generate-jwt --config config.toml --subject user_id
cargo fmt
cargo clippy --all-features -- -D warnings
just init        # Install tools
just pre-release <version>  # Generate changelog
```

## Architecture

### Entry Points
- `main.rs` (15 lines): Sets mimalloc, calls `app::run()`
- `lib.rs` (11 lines): Public exports: `aliyun`, `app`, `auth`, `error`
- `app.rs` (84 lines): CLI parser - `server`, `generate-jwt`, `version`

### AppState (src/state.rs)
- `bilibili_config: BilibiliConfig` - API credentials (sessdata, bili_jct)
- `aliyun_config: AliyunConfig` - OSS/CDN credentials
- `jwt_config: JwtConfig` - ES256 private/public keys
- `http_client: reqwest::Client` - Shared HTTP client
- **NO database or repository**

### Routes (all prefixed with `/api`)
**Public:**
- `GET /api/_ping` - Health check
- `GET /api/_health` - Health check
- `POST /api/aliyun/events` - OSS EventBridge (custom header auth)

**Protected (Bearer JWT):**
- `POST /api/bilibili/createDynamic` - Multipart file upload + dynamic post

**Docs:**
- `/api/scalar` - Scalar UI
- `/api/openapi.json` - OpenAPI spec

### Authentication
1. **Bilibili routes**: ES256 JWT via `Authorization: Bearer <token>` header
   - Token: `cargo run -- generate-jwt --config config.toml --subject user_id`
   - No expiration validation - long-lived tokens
2. **Aliyun routes**: Custom header `x-eventbridge-signature-token` (verified in handler)
   - Uses same JWT verification as Bilibili routes

### Error Handling (src/error.rs)
```rust
pub enum AppError {
    BadRequest(anyhow::Error),    // 400
    Unauthorized(anyhow::Error),  // 401
    InternalError(anyhow::Error),  // 500
}
// Response: { "code": 1 } (errors logged server-side)
```

## Configuration (example.toml)
- `logger`: enable, level (trace/debug/info/warn/error), format (compact/pretty/json)
- `server`: binding, port, host
- `bilibili`: sessdata, bili_jct
- `aliyun`: access_key_id, access_key_secret, bucket_url_map
- `jwt`: private_key, public_key (ES256 PEM)
- `sentry`: dsn, traces_sample_rate (optional)

## Anti-Patterns to Avoid

**Critical:**
- `.unwrap()`/`.expect()` in handlers (18 instances in bilibili_handlers.rs) - replace with Result

**Medium:**
- Excessive `.clone()` (15 instances) - use references
- String conversions (`.to_string()`) - use `&str` where ownership not needed

**CI Requirements:**
- `cargo fmt` must pass
- `cargo clippy --all-features -- -D warnings` must pass
- Code style: 4-space indentation for .rs (100 char limit)

## Module Organization
```
src/
├── main.rs           # CLI entry
├── lib.rs            # Public exports
├── app.rs            # CLI + server startup
├── config.rs         # TOML config
├── state.rs          # AppState
├── error.rs          # AppError
├── auth.rs           # JWT ES256
├── middleware.rs     # Tower layers (timeout, compression)
├── tracing.rs        # Logging setup
├── shutdown.rs       # Graceful shutdown
├── aliyun/          # OSS signature + CDN
│   ├── cdn.rs
│   └── signature.rs (with tests)
└── routes/           # HTTP handlers
    ├── bilibili_handlers.rs
    ├── aliyun_handlers.rs
    └── misc_handlers.rs
```

## Notes

**Architecture Gotchas:**
- NO database/SQLx (despite outdated README/AGENTS.md claims)
- NO migrations directory
- CI runs `rustfmt` + `clippy` ONLY (no tests)
- Only 3 tests exist (aliyun/signature.rs) - run manually with `cargo test`

**Rust 2024 Edition:**
- Uses experimental edition (pinned to 1.92.0)
- May have breaking changes from 2021 edition

**Release Process:**
1. `just pre-release <version>` - generates CHANGELOG.md via git-cliff
2. `cargo release <version>` - creates git tag
3. GitHub Action builds/pushes Docker image to GHCR
