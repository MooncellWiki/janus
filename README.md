# axum-template

This is an opinionated template for building a RESTful API via HTTP transport project using [axum](https://github.com/tokio-rs/axum) framework, with container support via [Docker](https://www.docker.com/) and CI/CD support via [GitHub Actions](https://github.com/features/actions).

## Default Configuration Structure

The configuration is divided into several main sections:

- Logger
- Server
- Database
- Mailer (SMTP)
- Sentry (Optional)

## Logger Configuration

Controls the application's logging behavior.

| Field             | Description                     | Options                                   |
| ----------------- | ------------------------------- | ----------------------------------------- |
| `enable`          | Enable log writing to stdout    | `true`/`false`                            |
| `level`           | Set logging level               | `trace`, `debug`, `info`, `warn`, `error` |
| `format`          | Set logger format               | `compact`, `pretty`, `json`               |
| `override_filter` | Override default tracing filter | Any valid tracing filter string           |

## Server Configuration

Configures the web server settings.

```toml
[server]
port = 25150
host = "http://localhost"
```

| Field     | Description                                      |
| --------- | ------------------------------------------------ |
| `binding` | Server binding address (defaults to "localhost") |
| `port`    | Port number for the server                       |
| `host`    | Web server host URL                              |

## Database Configuration

Database connection and pool settings.

```toml
[database]
uri = "postgres://user:password@localhost:5432/dbname"
```

| Field                        | Description                   | Default |
| ---------------------------- | ----------------------------- | ------- |
| `uri`                        | Database connection URI       | -       |
| `max_connections`            | Maximum database connections  | `None`  |
| `connection_timeout_seconds` | Connection timeout in seconds | `None`  |

## Mailer Configuration

Email sending configuration using SMTP.

```toml
[mailer.smtp]
host = "smtp.example.com"
port = 465

auth.user = "user@example.com"
auth.password = "password"
```

| Field           | Description                     |
| --------------- | ------------------------------- |
| `host`          | SMTP server host                |
| `port`          | SMTP server port                |
| `from_email`    | Email address to send from      |
| `to_email`      | Email address to send to        |
| `frontend_url`  | URL of the frontend application |
| `auth.user`     | SMTP authentication username    |
| `auth.password` | SMTP authentication password    |

## Sentry Configuration (Optional)

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

## Example Configuration

See the `example.toml` file for a complete example configuration.
