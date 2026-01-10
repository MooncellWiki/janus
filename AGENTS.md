# AGENTS.md

This file provides guidance to AI agent when working with code in this repository.

## Project Overview

**Janus** is a RESTful API service that provides Bilibili dynamic posting capabilities with JWT authentication and PostgreSQL persistence. The application:
- Posts content (text and images) to Bilibili dynamics via API integration
- Uses ES256 JWT authentication for API security
- Stores data in PostgreSQL with SQLx (compile-time checked queries)
- Auto-generates OpenAPI documentation with Scalar UI
- Supports multipart file uploads for images

**Tech Stack:**
- **Axum 0.8** web framework with Tower middleware
- **PostgreSQL** with SQLx for database operations
- **Utoipa** for OpenAPI documentation
- **Reqwest** HTTP client for Bilibili API calls
- **ES256 JWT** (ECDSA with P-256) for authentication
- **Tracing** subscriber for structured logging
- **Sentry** for optional error tracking

## Build and Development Commands

### Essential Commands
```bash
# Build the project
cargo build

# Run server (requires config.toml)
cargo run -- server --config config.toml

# Generate JWT token for authentication
cargo run -- generate-jwt --config config.toml --subject user_id

# Show version and build SHA
cargo run -- version

# Format code
cargo fmt

# Run linter (strict mode - warnings as errors)
cargo clippy --all-features -- -D warnings
```

### Testing
```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture
```

### Database Operations
```bash
# Run migrations (requires sqlx-cli)
sqlx migrate run

# Install sqlx-cli if not installed
cargo install sqlx-cli
```

### Just Commands
```bash
# Initialize development tools
just init

# Run database migrations
just up

# Generate changelog before release
just pre-release <version>
```

## Architecture

### Configuration System
All configuration is TOML-based and loaded via `config.rs`. The config file path is passed via CLI argument `--config`. Configuration structure (`AppSettings`):
- **Logger**: enable, level (trace/debug/info/warn/error), format (compact/pretty/json)
- **Server**: binding address, port, and host URL
- **Database**: PostgreSQL URI with max connections and timeout
- **Smtp**: Optional SMTP configuration for emails
- **Sentry**: Optional DSN and traces_sample_rate
- **Bilibili**: sessdata and bili_jct cookies for API authentication
- **Jwt**: ES256 private/public keys in PEM format

### Application Flow
1. `main.rs`: Entry point using mimalloc global allocator
2. `app.rs::run()`: CLI parser (`clap`) with three commands:
   - `server --config <path>`: Load config, initialize tracing/Sentry, start web server
   - `generate-jwt --config <path> --subject <id>`: Generate ES256 JWT token
   - `version`: Display version and build SHA
3. Server initialization sequence:
   - Load TOML config via `AppSettings::new()`
   - Initialize tracing subscriber with module whitelist: `["tower_http", "sqlx::query", "my_axum_template"]`
   - Initialize Sentry (if configured)
   - Create `AppState` with PostgreSQL pool, shared HTTP client, and configs
   - Run database migrations automatically via `repository.migrate()`
   - Build Axum router with OpenAPI support
   - Apply Tower middleware (timeout, compression)
   - Bind TCP listener and start server with graceful shutdown handling

### State Management
`AppState` (src/state.rs) holds application-wide state:
- `repository: PostgresRepository` - Database operations with SQLx pool
- `bilibili_config: BilibiliConfig` - Bilibili API credentials
- `jwt_config: JwtConfig` - JWT signing/verification keys
- `http_client: reqwest::Client` - Shared HTTP client for Bilibili API calls

Repository trait provides:
- `health_check()`: Database connectivity check
- `migrate()`: Run SQLx migrations from `./migrations` directory

### Routing Structure
Routes are organized in `src/routes/` using Utoipa's OpenApiRouter:
- All API routes prefixed with `/api` (note: NOT `/api/v1`)
- **Public routes** (no auth required):
  - `GET /api/_ping` - Simple ping endpoint
  - `GET /api/_health` - Database health check
- **Protected routes** (JWT required):
  - `POST /api/bilibili/createDynamic` - Create Bilibili dynamic with multipart file upload
- OpenAPI documentation available at:
  - `/api/scalar` - Scalar UI
  - `/api/openapi.json` - OpenAPI spec JSON
- JWT auth middleware applied via `middleware::from_fn_with_state()` to protected routes
- Security scheme: HTTP Bearer with JWT format

### Authentication
JWT-based authentication using ES256 algorithm (ECDSA with P-256):
- `Claims` structure: `sub` (subject) and `iat` (issued at timestamp)
- No expiration validation (`validate_exp = false`) - tokens are long-lived
- Token generation: `generate_token(subject, private_key_pem)` via CLI command
- Token verification: `verify_token(token, public_key_pem)`
- Middleware: `jwt_auth_middleware()` extracts `Authorization: Bearer <token>` header

### Middleware Layer
Applied in `src/middleware.rs` via Tower layers:
- **RequestBodyTimeoutLayer**: 10-second timeout on request body
- **CompressionLayer**: Response compression (tower-http compression-full)

### Error Handling
Custom `AppError` enum (src/error.rs) with three variants:
- `BadRequest(anyhow::Error)` → HTTP 400
- `Unauthorized(anyhow::Error)` → HTTP 401
- `InternalError(anyhow::Error)` → HTTP 500

Response format: `{ "code": 1 }` (errors logged server-side with details)
Automatic conversions from: `sqlx::Error`, `serde_json::Error`, `reqwest::Error`, `anyhow::Error`
`AppResult<T>` type alias: `Result<T, AppError>`

### Testing
- CI runs `rustfmt` and `clippy` checks
- `SQLX_OFFLINE=true` is set in CI to allow offline builds (requires `sqlx-data.json` files)
- No dedicated test files in codebase (relies on manual testing via HTTP clients)

## Key Development Notes

### Adding New Routes
1. Create handler function in appropriate module under `src/routes/`:
   ```rust
   use crate::{error::AppResult, state::AppState};
   use axum::{Json, extract::State};
   use utoipa::ToSchema;

   #[derive(ToSchema, Serialize)]
   pub struct MyResponse { pub field: String }

   #[utoipa::path(
       post,
       tag = "mytag",
       path = "/myendpoint",
       request_body(...),
       responses(...),
       security(("bearer_auth" = []))
   )]
   pub async fn my_handler(
       State(state): State<AppState>,
   ) -> AppResult<Json<MyResponse>> {
       Ok(Json(MyResponse { field: "value".to_string() }))
   }
   ```
2. Register route in `src/routes/mod.rs`:
   - For public routes: Add before `.route_layer(jwt_auth_middleware)`
   - For protected routes: Add after `.route_layer(jwt_auth_middleware)`
3. Add tag to `ApiDoc` struct's `tags()` macro if creating new tag
4. Routes automatically prefixed with `/api` and documented in OpenAPI spec

### Database Queries
- Use SQLx with compile-time query verification
- Access pool via `state.repository.pool` (it's public but used directly in queries)
- Place migration files in `./migrations` directory
- Create migrations: `sqlx migrate add -r migration_name`
- Repository trait allows for alternative database backends

### Error Handling in Handlers
```rust
use crate::error::{AppError, AppResult};

pub async fn my_handler() -> AppResult<Json<Response>> {
    // Use ? operator for automatic conversion
    let data = risky_operation()?;

    // Or use AppError explicitly
    if invalid {
        return Err(AppError::BadRequest(anyhow::anyhow!("Invalid input")));
    }

    Ok(Json(data))
}
```

### Logging
Use `tracing` macros, not `println!` or `log!`:
```rust
use tracing::info;
info!("Creating dynamic with {} images", image_count);
```

### Memory Allocation
The application uses **mimalloc** as the global allocator (configured in `main.rs`) for improved performance.

### Graceful Shutdown
Implemented via `shutdown.rs` using Tokio signals, ensuring clean connection closure on termination.
