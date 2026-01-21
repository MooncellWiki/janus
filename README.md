# Janus

A stateless RESTful API gateway that posts to Bilibili dynamics, receives Aliyun OSS EventBridge webhooks, and refreshes Aliyun CDN. Built with [Axum](https://github.com/tokio-rs/axum) framework, with container support via [Docker](https://www.docker.com/) and CI/CD support via [GitHub Actions](https://github.com/features/actions).

## Features

- **Stateless architecture** - No database, pure HTTP API service
- **Bilibili integration** - Post dynamics with multipart file upload
- **Aliyun OSS EventBridge** - Receive and process OSS events via webhooks
- **Aliyun CDN refresh** - Cache invalidation support
- **JWT authentication** - ES256 (ECDSA P-256) for protected routes
- **OpenAPI documentation** - Scalar UI at `/api/scalar`
- **mimalloc allocator** - High-performance memory allocation

## Tech Stack

- **Axum 0.8** + Tower middleware
- **Reqwest** for external APIs
- **Utoipa** for OpenAPI spec generation
- **ES256 JWT** (ECDSA P-256) for authentication
- **Aliyun V3 signature** for OSS EventBridge

## Configuration

The configuration is divided into several main sections:

- Logger
- Server
- Bilibili (API credentials)
- Aliyun (OSS/CDN credentials)
- JWT (ES256 keys)
- Sentry (Optional)

### Logger Configuration

Controls the application's logging behavior.

| Field             | Description                     | Options                                   |
| ----------------- | ------------------------------- | ----------------------------------------- |
| `enable`          | Enable log writing to stdout    | `true`/`false`                            |
| `level`           | Set logging level               | `trace`, `debug`, `info`, `warn`, `error` |
| `format`          | Set logger format               | `compact`, `pretty`, `json`               |
| `override_filter` | Override default tracing filter | Any valid tracing filter string           |

### Server Configuration

Configures the web server settings.

```toml
[server]
binding = "0.0.0.0"
port = 25150
host = "http://localhost"
```

| Field     | Description                                      |
| --------- | ------------------------------------------------ |
| `binding` | Server binding address (defaults to "0.0.0.0")  |
| `port`    | Port number for the server                       |
| `host`    | Web server host URL                              |

### Bilibili Configuration

Bilibili API credentials for posting dynamics.

```toml
[bilibili]
sessdata = "your_bilibili_sessdata_cookie"
bili_jct = "your_bilibili_bili_jct"
```

| Field       | Description                                    |
| ----------- | ---------------------------------------------- |
| `sessdata`  | Bilibili SESSDATA cookie value                 |
| `bili_jct`  | Bilibili bili_jct cookie value (for CSRF)      |

### Aliyun Configuration

Aliyun OSS and CDN credentials.

```toml
[aliyun]
access_key_id = "your_aliyun_access_key_id"
access_key_secret = "your_aliyun_access_key_secret"

[aliyun.bucket_url_map]
prts-static = "https://static.prts.wiki/{object_key}"
ak-media = "https://media.prts.wiki/{object_key}"
```

| Field                | Description                                    |
| -------------------- | ---------------------------------------------- |
| `access_key_id`      | Aliyun Access Key ID                          |
| `access_key_secret`  | Aliyun Access Key Secret                      |
| `bucket_url_map`     | Bucket to URL template mapping (optional)      |

The `{object_key}` placeholder in `bucket_url_map` will be URL-encoded and replaced with the actual object key.

### JWT Configuration

ES256 (ECDSA P-256) keys for API authentication.

```toml
[jwt]
private_key = """-----BEGIN EC PRIVATE KEY-----
YOUR_PRIVATE_KEY_HERE
-----END EC PRIVATE KEY-----"""
public_key = """-----BEGIN PUBLIC KEY-----
YOUR_PUBLIC_KEY_HERE
-----END PUBLIC KEY-----"""
```

| Field         | Description                          |
| ------------- | ------------------------------------ |
| `private_key` | ES256 private key (PEM format)       |
| `public_key`  | ES256 public key (PEM format)        |

#### Generating ES256 Key Pair

```bash
# Generate private key (prime256v1 = P-256)
openssl ecparam -genkey -name prime256v1 -noout -out private.pem

# Extract public key
openssl ec -in private.pem -pubout -out public.pem
```

### Sentry Configuration (Optional)

Optional configuration for Sentry error tracking and monitoring.

```toml
[sentry]
dsn = "https://your-sentry-dsn@sentry.io/project-id"
traces_sample_rate = 1.0
```

| Field                | Description                                    | Required |
| -------------------- | ---------------------------------------------- | -------- |
| `dsn`                | Sentry DSN for error reporting                 | No       |
| `traces_sample_rate` | Sampling rate for performance traces (0.0-1.0) | No       |

## API Endpoints

### Public Routes

| Method | Path          | Description               |
| ------ | ------------- | ------------------------- |
| GET    | `/api/_ping`  | Health check (ping)       |
| GET    | `/api/_health`| Health check (detailed)   |
| POST   | `/api/aliyun/events` | OSS EventBridge webhook |

### Protected Routes (Bearer JWT)

All protected routes require `Authorization: Bearer <token>` header.

| Method | Path                    | Description                     |
| ------ | ----------------------- | ------------------------------- |
| POST   | `/api/bilibili/createDynamic` | Create Bilibili dynamic with file upload |

### Documentation

| Path                 | Description           |
| -------------------- | --------------------- |
| `/api/scalar`        | Scalar UI (OpenAPI)  |
| `/api/openapi.json`  | OpenAPI specification |

## Authentication

### Bilibili Routes

Protected routes use ES256 JWT. Generate a token:

```bash
cargo run -- generate-jwt --config config.toml --subject user_id
```

Use the token in requests:

```bash
curl -X POST http://localhost:25150/api/bilibili/createDynamic \
  -H "Authorization: Bearer <token>" \
  -F "file=@/path/to/image.jpg" \
  -F "text=Hello Bilibili"
```

### Aliyun Routes

EventBridge webhooks use a custom header `x-eventbridge-signature-token` for authentication, verified using the same JWT verification as Bilibili routes.

## Commands

```bash
# Build the project
cargo build

# Run the server
cargo run -- server --config config.toml

# Generate a JWT token
cargo run -- generate-jwt --config config.toml --subject user_id

# Format code
cargo fmt

# Run linter (must pass in CI)
cargo clippy --all-features -- -D warnings

# Install development tools
just init

# Generate changelog for release
just pre-release <version>
```

## Development

### Code Style

- 4-space indentation for Rust files
- 100 character line limit
- Run `cargo fmt` before committing
- Run `cargo clippy --all-features -- -D warnings` (CI enforces this)

### Testing

```bash
# Run tests
cargo test
```

Note: Only a few tests exist (aliyun/signature.rs). CI does not run tests automatically.

### Release Process

1. `just pre-release <version>` - Generate CHANGELOG.md via git-cliff
2. `cargo release <version>` - Create git tag
3. GitHub Action builds and pushes Docker image to GHCR

## Example Configuration

See the `example.toml` file for a complete example configuration.

## Architecture

### Entry Points

- `main.rs` (15 lines): Sets mimalloc, calls `app::run()`
- `lib.rs` (11 lines): Public exports: `aliyun`, `app`, `auth`, `error`
- `app.rs` (84 lines): CLI parser - `server`, `generate-jwt`, `version`

### AppState (src/state.rs)

- `bilibili_config: BilibiliConfig` - API credentials
- `aliyun_config: AliyunConfig` - OSS/CDN credentials
- `jwt_config: JwtConfig` - ES256 private/public keys
- `http_client: reqwest::Client` - Shared HTTP client
- **NO database or repository**

### Module Organization

```
src/
├── main.rs           # CLI entry
├── lib.rs            # Public exports
├── app.rs            # CLI + server startup
├── config.rs         # TOML config
├── state.rs          # AppState
├── error.rs          # AppError
├── auth.rs           # JWT ES256
├── middleware.rs     # Tower layers
├── tracing.rs        # Logging setup
├── shutdown.rs       # Graceful shutdown
├── aliyun/          # OSS signature + CDN
│   ├── cdn.rs
│   └── signature.rs
└── routes/           # HTTP handlers
    ├── bilibili_handlers.rs
    ├── aliyun_handlers.rs
    └── misc_handlers.rs
```

## License

MIT
