# AGENTS.md

This file provides guidance to AI agent when working with code in this repository.

## Project Overview

This is an Axum-based RESTful API template with PostgreSQL support, Docker containerization, and GitHub Actions CI/CD. The application uses:
- **Axum 0.8** web framework with Tower middleware
- **PostgreSQL** with SQLx for database operations (compile-time checked queries)
- **Utoipa** for OpenAPI documentation with Scalar UI
- **Sentry** for optional error tracking
- **Tracing** subscriber for structured logging

## Build and Development Commands

### Building and Testing
```bash
# Build the project
cargo build

# Run with config file
cargo run -- server --config config.toml

# Format code
cargo fmt

# Run linter
cargo clippy --all-features -- -D warnings

# Show version
cargo run -- version
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
All configuration is TOML-based and loaded via `config.rs`. The config file path is passed via CLI argument `--config`. Configuration structure:
- **Logger**: tracing-subscriber with configurable level (trace/debug/info/warn/error) and format (compact/pretty/json)
- **Server**: binding address, port, and host URL
- **Database**: PostgreSQL URI with optional connection pool settings
- **Mailer**: Optional SMTP configuration for emails
- **Sentry**: Optional error tracking with DSN and sampling rate

### Application Flow
1. `main.rs`: Entry point using mimalloc global allocator
2. `app.rs::run()`: CLI parser (`clap`) with two commands:
   - `server`: Loads config, initializes tracing/Sentry, starts web server
   - `version`: Displays version and build SHA
3. Server initialization sequence:
   - Load TOML config
   - Initialize tracing (based on logger config)
   - Initialize Sentry (if configured)
   - Create AppState with PostgreSQL pool
   - Run database migrations
   - Build Axum router with middleware
   - Start server with graceful shutdown handling

### State Management
`AppState` (src/state.rs) holds the `PostgresRepository` with a SQLx connection pool. The repository pattern provides:
- `health_check()`: Database connectivity check
- `migrate()`: Run SQLx migrations from `./migrations` directory

### Routing Structure
Routes are organized in `src/routes/` using Utoipa's OpenApiRouter:
- All API routes are prefixed with `/api/v1`
- OpenAPI documentation available at:
  - `/api/v1/scalar` - Scalar UI
  - `/api/v1/openapi.json` - OpenAPI spec JSON
- Route handlers use `utoipa` macros for automatic OpenAPI spec generation

### Middleware Layer
Applied in `src/middleware.rs` via Tower:
- **CompressionLayer**: Response compression (tower-http compression-full)
- **RequestBodyTimeoutLayer**: 10-second request timeout

### Error Handling
- **anyhow**: Used in `app.rs` for main application errors
- **thiserror**: Used in `config.rs` for typed config errors
- **SQLx**: Database errors propagate as Result types

### Testing
- CI runs `rustfmt` and `clippy` checks
- `SQLX_OFFLINE=true` is set in CI to allow offline builds (requires `sqlx-data.json` files)

## Key Development Notes

### Adding New Routes
1. Create handler functions in `src/routes/` modules
2. Add `utoipa` OpenAPI macros to handlers
3. Register routes in `build_router()` using `routes!()` macro
4. Routes are automatically prefixed with `/api/v1` and documented

### Database Queries
- Use SQLx with compile-time query verification
- Place migration files in `./migrations` directory
- Repository trait allows for alternative database backends
- All database operations go through `PostgresRepository`

### Memory Allocation
The application uses **mimalloc** as the global allocator (configured in `main.rs`) for improved performance.

### Graceful Shutdown
Implemented via `shutdown.rs` using Tokio signals, ensuring clean connection closure on termination.
