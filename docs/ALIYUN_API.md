# Aliyun OSS EventBridge API

This document describes the Aliyun OSS EventBridge webhook functionality implemented in this project.

## Overview

The `POST /api/aliyun/events` endpoint receives OSS event notifications from Aliyun EventBridge and automatically refreshes the corresponding CDN cache for the modified object.

- **Authentication:** Required. The route is protected by JWT verification via the `x-eventbridge-signature-token` header.
- **Implementation:** Axum JSON parsing + `reqwest` calls to Aliyun CDN API using V3 signature (ACS3-HMAC-SHA256).

## Configuration

Add the following sections to your `config.toml`:

```toml
[aliyun]
access_key_id = "your_aliyun_access_key_id"
access_key_secret = "your_aliyun_access_key_secret"

[aliyun.bucket_url_map]
my-bucket = "https://cdn.example.com/{object_key}"
another-bucket = "https://static.example.com/{object_key}"

[jwt]
private_key = """-----BEGIN EC PRIVATE KEY-----
YOUR_PRIVATE_KEY_HERE
-----END EC PRIVATE KEY-----"""
public_key = """-----BEGIN PUBLIC KEY-----
YOUR_PUBLIC_KEY_HERE
-----END PUBLIC KEY-----"""
```

### Configuration Fields

**Aliyun Config:**
- **access_key_id** (required): Your Aliyun Access Key ID.
- **access_key_secret** (required): Your Aliyun Access Key Secret. Used for V3 signature generation.
- **bucket_url_map** (required): Mapping from OSS bucket names to CDN URL templates. The `{object_key}` placeholder will be replaced with the URL-encoded object key.

**JWT Config:**
- **private_key** (required): ES256 private key in PEM format for signing JWT tokens.
- **public_key** (required): ES256 public key in PEM format for verifying JWT tokens.

### Generating JWT Keys

Generate ES256 key pair using OpenSSL:

```bash
openssl ecparam -genkey -name prime256v1 -noout -out private.pem
openssl ec -in private.pem -pubout -out public.pem
```

### Obtaining Aliyun Credentials

1. Log in to the [Aliyun Console](https://console.aliyun.com/)
2. Navigate to **AccessKey Management**
3. Create an AccessKey or use an existing one
4. Note the **AccessKey ID** and **AccessKey Secret**

**Important:** The AccessKey must have permissions for:
- **OSS** (to receive EventBridge events - managed by Aliyun)
- **CDN** (to refresh object caches)

## Generating JWT Tokens

Generate a JWT token using the CLI command:

```bash
cargo run -- generate-jwt --config config.toml --subject eventbridge_user
```

Options:
- `--config`: Path to config file (default: `config.toml`)
- `--subject`: Subject identifier (e.g., user ID or service name)

Notes:
- Tokens are ES256 signed.
- This implementation does not validate `exp` (no expiration claim is required/checked).

## API Endpoint

### POST `/api/aliyun/events`

Receives OSS EventBridge events and triggers CDN cache refresh for the modified object.

**Authentication:** Required via `x-eventbridge-signature-token: <jwt_token>` header.

#### Request

**Headers:**
- **x-eventbridge-signature-token** (required): JWT token for authentication (same format as Bilibili routes)

**Content-Type:** `application/json`

**Request Body:**

The request body follows Aliyun EventBridge CloudEvents format with OSS-specific data:

```json
{
  "id": "event-id",
  "source": "acs.oss",
  "specversion": "1.0",
  "type": "oss:ObjectCreated:PostObject",
  "datacontenttype": "application/json",
  "subject": "acs:oss:cn-shanghai:123456789:bucket-name/object-key",
  "time": "2023-10-26T10:22:32Z",
  "data": {
    "region": "cn-shanghai",
    "eventVersion": "1.0",
    "eventSource": "acs:oss",
    "eventName": "ObjectCreated:PostObject",
    "eventTime": "2023-10-26T10:22:32Z",
    "oss": {
      "bucket": {
        "name": "my-bucket",
        "arn": "acs:oss:cn-shanghai:123456789:bucket/my-bucket",
        "ownerIdentity": "123456789"
      },
      "object": {
        "key": "path/to/object.jpg",
        "eTag": "d41d8cd98f00b204e9800998ecf8427e",
        "deltaSize": 1024
      },
      "ossSchemaVersion": "1.0"
    }
  }
}
```

#### Request Examples

**Test with curl:**

```bash
curl -X POST http://localhost:25150/api/aliyun/events \
  -H "x-eventbridge-signature-token: YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "id": "test-event-123",
    "source": "acs.oss",
    "type": "oss:ObjectCreated:PostObject",
    "time": "2023-10-26T10:22:32Z",
    "data": {
      "oss": {
        "bucket": {
          "name": "my-bucket"
        },
        "object": {
          "key": "images/photo.jpg"
        }
      }
    }
  }'
```

#### Response Format

**Success Response (HTTP 200):**

```json
{
  "message": "CDN refresh triggered for images/photo.jpg in bucket my-bucket",
  "task_id": "1234567890"
}
```

Fields:
- **message**: Description of what action was taken
- **task_id**: CDN refresh task ID returned by Aliyun API (can be used to track refresh status)

**Error Response:**

- Auth failures return **HTTP 401** with body `{ "code": 1 }`.
- Validation / Aliyun failures return JSON with body `{ "code": 1 }`. No additional error message fields are included in the response; detailed error information is only logged internally.

#### Error Codes (Conceptual)

| HTTP | code | Description |
|------|------|-------------|
| 401 | 1 | Missing or invalid `x-eventbridge-signature-token` header |
| 401 | 1 | JWT verification failed |
| 400 | 1 | Failed to parse OSS event payload (invalid JSON) |
| 400 | 1 | Unsupported bucket (bucket not in `bucket_url_map`) |
| 500 | 1 | Failed to send CDN refresh request to Aliyun |
| 500 | 1 | Aliyun CDN API error (non-success status) |
| 500 | 1 | Failed to parse CDN refresh response |

Note:
- These descriptions explain when errors occur, but the actual HTTP response body is always `{ "code": 1 }` for failures.

## Event Payload Structure

### CloudEvents Envelope

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | string | Yes | Event ID |
| `source` | string | Yes | Event source (e.g., `acs.oss`) |
| `specversion` | string | No | CloudEvents specification version |
| `type` | string | No | Event type (e.g., `oss:ObjectCreated:PostObject`) |
| `datacontenttype` | string | No | Content type of the data field |
| `subject` | string | No | Event subject identifier |
| `time` | string | No | Event timestamp (ISO 8601 format) |
| `data` | OssEventData | Yes | OSS-specific event data |

### OSS Event Data

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `region` | string | No | OSS region (e.g., `cn-shanghai`) |
| `eventVersion` | string | No | Event schema version |
| `eventSource` | string | No | Event source identifier |
| `eventName` | string | No | Event name (e.g., `ObjectCreated:PostObject`) |
| `eventTime` | string | No | Event timestamp |
| `oss` | OssData | Yes | OSS bucket and object information |

### Bucket Information

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | Yes | OSS bucket name (used to look up URL template in config) |
| `arn` | string | No | Bucket ARN |
| `ownerIdentity` | string | No | Bucket owner ID |

### Object Information

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `key` | string | Yes | Object key (path within bucket) |
| `eTag` | string | No | Object ETag |
| `deltaSize` | integer | No | Size change in bytes |

## Implementation Details

### Event Processing Flow

1. Client sends EventBridge webhook with JWT in `x-eventbridge-signature-token` header
2. Server validates JWT authentication using the public key from configuration
3. Server parses the EventBridge payload (CloudEvents format with OSS data)
4. Server extracts bucket name and object key from the event
5. Server looks up the CDN URL template for the bucket from `bucket_url_map` config
6. Server URL-encodes the object key and replaces `{object_key}` placeholder in template
7. Server creates Aliyun CDN client with V3 signature capability
8. Server calls `RefreshObjectCaches` API to invalidate the CDN cache
9. Server returns success response with CDN task ID

### CDN Refresh Request

The endpoint calls Aliyun CDN's `RefreshObjectCaches` API with the following parameters:

- **ObjectPath**: The full URL to refresh (constructed from bucket URL template + encoded object key)
- **ObjectType**: `"File"` (file-level refresh)
- **Force**: `false` (don't force delete cache nodes, just mark as expired)

Reference: [RefreshObjectCaches API](https://help.aliyun.com/zh/cdn/developer-reference/api-cdn-2018-05-10-refreshobjectcaches)

### Aliyun V3 Signature

The CDN client uses Aliyun OpenAPI V3 signature (ACS3-HMAC-SHA256):

- Signature method: `ACS3-HMAC-SHA256`
- Canonical request includes: method, URI, query string, headers, signed headers, body hash
- Headers included in signature: `host`, `content-type`, all `x-acs-*` headers
- Body content is hashed using SHA-256 for integrity

Reference: [V3 Request Structure and Signature](https://help.aliyun.com/zh/sdk/product-overview/v3-request-structure-and-signature)

### Bucket URL Template Mapping

The `bucket_url_map` in configuration maps bucket names to CDN URL templates:

```toml
[aliyun.bucket_url_map]
my-bucket = "https://cdn.example.com/{object_key}"
```

The `{object_key}` placeholder is replaced with:
1. URL-encoded object key (RFC 3986 unreserved chars: `-`, `_`, `.`, `~` are NOT encoded)
2. Spaces become `%20`, special chars are percent-encoded

**Examples:**

| Object Key | Template | Result URL |
|-----------|----------|-----------|
| `images/photo.jpg` | `https://cdn.example.com/{object_key}` | `https://cdn.example.com/images/photo.jpg` |
| `path/to/file.png` | `https://cdn.example.com/{object_key}` | `https://cdn.example.com/path/to/file.png` |
| `file with spaces.pdf` | `https://cdn.example.com/{object_key}` | `https://cdn.example.com/file%20with%20spaces.pdf` |
| `valid-name_1.0~file` | `https://cdn.example.com/{object_key}` | `https://cdn.example.com/valid-name_1.0~file` |

### Authentication Flow

The endpoint uses the same JWT authentication mechanism as Bilibili routes:

1. **JWT Authentication**: Protected via `x-eventbridge-signature-token` header
   - Token must be a valid ES256-signed JWT
   - Tokens are verified using the public key from configuration
   - No expiration validation (long-lived tokens)
2. **Aliyun CDN API**: Uses V3 signature (ACS3-HMAC-SHA256) for API calls
   - Signature generated using Access Key ID and Secret
   - Automatic timestamp and nonce generation for each request

### HTTP Client

The implementation uses a shared `reqwest::Client` instance stored in `AppState` for efficient connection pooling and reuse across CDN API requests.

## Setting Up EventBridge Rules

To have Aliyun OSS send events to this endpoint:

1. **Create an EventBridge Rule:**
   - Log in to Aliyun Console
   - Navigate to **EventBridge** → **Event Rules**
   - Create a new rule with custom event pattern

2. **Configure Event Pattern:**
   ```json
   {
     "source": ["acs.oss"],
     "type": ["oss:ObjectCreated:*"]
   }
   ```

3. **Set Event Target:**
   - Target type: **HTTP Service (Private)**
   - URL: `http://your-server:25150/api/aliyun/events`
   - Authentication: Add custom header
     - Header name: `x-eventbridge-signature-token`
     - Header value: Your JWT token

4. **Configure Event Bus:**
   - Use the default event bus or create a custom one
   - Ensure OSS service is enabled as event source

**Note:** For production deployments, ensure:
- The endpoint is publicly accessible (or via VPN/VPC)
- HTTPS is used (if possible, via load balancer)
- JWT tokens are rotated periodically
- EventBridge delivery retries are configured

## OpenAPI Documentation

The API is documented with OpenAPI. Access the interactive documentation at:

- **Scalar UI**: `http://localhost:25150/api/scalar`
- **OpenAPI JSON**: `http://localhost:25150/api/openapi.json`

## Notes on Current Implementation

- Only file-level refresh is supported (`ObjectType: File`). Directory refresh is not implemented.
- Only single object refresh per event. Batch refresh is not supported.
- The endpoint does not verify that the event is genuine from Aliyun (only JWT auth is checked). In production, consider adding additional signature verification if your EventBridge endpoint is public.
- URL encoding follows RFC 3986 unreserved character rules (same as Aliyun's own implementations).
- CDN refresh uses `force: false`, meaning cache nodes are marked as expired but not immediately deleted.

## Deployment Considerations

### Security

- Keep your `config.toml` file secure and never commit it to version control
- Use environment-specific configuration files
- Rotate JWT tokens periodically if the endpoint is public
- Consider adding IP allow-listing for EventBridge webhooks
- Use HTTPS in production (via reverse proxy or load balancer)
- Aliyun Access Keys should have minimum required permissions (OSS read for EventBridge, CDN write for refresh)

### Performance

- The shared HTTP client provides connection pooling for efficient CDN API calls
- Each event triggers exactly one CDN refresh request (synchronous)
- CDN refresh is non-blocking from the client perspective (Aliyun processes asynchronously)
- Consider rate limiting if receiving high-volume event streams

### Error Handling

- All errors are logged using the tracing framework
- Failed CDN refresh requests return appropriate error responses
- Network timeouts are handled gracefully
- EventBridge can be configured to retry failed deliveries

### Monitoring

- Monitor CDN refresh task IDs returned in responses
- Use Aliyun CDN console to track refresh status and quotas
- Monitor logs for authentication failures
- Track unsupported bucket errors (may indicate config drift)

## Troubleshooting

### JWT Token Issues
- Ensure the token is passed in the `x-eventbridge-signature-token` header
- Check that the public/private key pair in config matches the keys used to generate the token
- Regenerate token using the CLI command if needed
- Verify token is not malformed (should be three parts separated by dots)

### "Unsupported bucket" error
- Check that the bucket name from the event exists in `bucket_url_map` configuration
- Verify bucket name spelling and case sensitivity
- Ensure configuration is properly loaded on server startup

### "Failed to parse OSS event payload" error
- Verify the request body is valid JSON
- Check that required fields (`data.oss.bucket.name`, `data.oss.object.key`) are present
- Ensure Content-Type header is set to `application/json`

### CDN refresh not working
- Verify Aliyun Access Key has CDN permissions
- Check CDN refresh quota limits (Aliyun has daily limits)
- Verify the constructed URL template is correct (check object key encoding)
- Use CDN task ID to check refresh status in Aliyun console
- Verify bucket URL template has correct `{object_key}` placeholder

### Events not being received
- Check EventBridge rule configuration
- Verify event pattern matches OSS events
- Ensure event target URL is correct and accessible
- Check EventBridge delivery logs
- Verify authentication header is configured correctly in EventBridge target

### URL encoding issues
- Object keys with special characters should be properly encoded
- Unreserved characters (`-`, `_`, `.`, `~`) are NOT encoded
- Spaces are encoded as `%20`
- Chinese and other non-ASCII characters are percent-encoded

## Example Integration

### Using with JavaScript/TypeScript

```typescript
async function triggerCdnRefresh(eventData: any, jwtToken: string) {
  const response = await fetch('http://localhost:25150/api/aliyun/events', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'x-eventbridge-signature-token': jwtToken
    },
    body: JSON.stringify(eventData)
  });

  const result = await response.json();
  console.log('CDN refresh task ID:', result.task_id);
  return result;
}

// Example usage:
const ossEvent = {
  id: '123',
  source: 'acs.oss',
  type: 'oss:ObjectCreated:PostObject',
  data: {
    oss: {
      bucket: {
        name: 'my-bucket'
      },
      object: {
        key: 'uploads/image.jpg'
      }
    }
  }
};

triggerCdnRefresh(ossEvent, 'YOUR_JWT_TOKEN');
```

### Using with Python

```python
import requests
import json

def trigger_cdn_refresh(event_data, jwt_token):
    url = 'http://localhost:25150/api/aliyun/events'

    headers = {
        'Content-Type': 'application/json',
        'x-eventbridge-signature-token': jwt_token
    }

    response = requests.post(url, headers=headers, json=event_data)
    result = response.json()

    print(f'CDN refresh task ID: {result.get("task_id")}')
    return result

# Example usage:
oss_event = {
    'id': '123',
    'source': 'acs.oss',
    'type': 'oss:ObjectCreated:PostObject',
    'data': {
        'oss': {
            'bucket': {
                'name': 'my-bucket'
            },
            'object': {
                'key': 'uploads/image.jpg'
            }
        }
    }
}

trigger_cdn_refresh(oss_event, 'YOUR_JWT_TOKEN')
```

## Development

### Building

```bash
cargo build --release
```

### Running

```bash
cargo run -- server --config config.toml
```

### Testing

The endpoint can be tested with curl, Postman, or any HTTP client. See the request examples above for sample payloads.

To simulate EventBridge events for testing:

1. Generate a JWT token
2. Prepare an OSS event payload
3. Send POST request to `/api/aliyun/events`
4. Verify CDN refresh task ID is returned

## License

This implementation follows the same license as the main project.
