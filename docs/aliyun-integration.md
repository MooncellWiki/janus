# Alibaba Cloud CDN API Integration

This document explains how to use the Alibaba Cloud CDN API integration in Janus.

## Overview

The implementation provides:
1. **Alibaba Cloud V3 API signature algorithm** using HMAC-SHA256 (`src/aliyun/signature.rs`)
2. **CDN client** for making API calls (`src/aliyun/cdn.rs`)
3. **DescribeRefreshTasks API** endpoint for querying refresh task status

## Configuration

Add the following section to your `config.toml`:

```toml
[aliyun]
access_key_id = "your_access_key_id"
access_key_secret = "your_access_key_secret"
cdn_endpoint = "cdn.aliyuncs.com"  # Optional, defaults to "cdn.aliyuncs.com"
```

**Note:** The `aliyun` section is optional. If not provided, the Alibaba Cloud features will not be available.

## API Endpoint

### POST /api/aliyun/cdn/describeRefreshTasks

Query the status of CDN refresh or prefetch tasks.

**Authentication:** Requires JWT Bearer token in Authorization header.

**Request Body (all fields optional):**

```json
{
  "task_id": "1234567890",
  "object_path": "http://example.com/path/to/file.jpg",
  "page_number": 1,
  "object_type": "file",
  "domain_name": "example.com",
  "status": "Complete",
  "page_size": 20,
  "start_time": "2023-12-10T08:00:00Z",
  "end_time": "2023-12-12T08:00:00Z",
  "resource_group_id": "rg-xxxxx"
}
```

**Field Descriptions:**
- `task_id`: The ID of the refresh or prefetch task
- `object_path`: The path (URL) of the object for an exact match
- `page_number`: The page number for paginated results (1-100000)
- `object_type`: The type of task - `file`, `directory`, `preload`, `regex`, `block`, `unblock`
- `domain_name`: The accelerated domain name
- `status`: Task status - `Complete`, `Refreshing`, `Failed`
- `page_size`: Number of entries per page (default: 20, max: 100)
- `start_time`: Start of time range (ISO8601 UTC format)
- `end_time`: End of time range (ISO8601 UTC format)
- `resource_group_id`: The resource group ID

**Response:**

```json
{
  "code": 0,
  "data": {
    "RequestId": "A8B0D7A5-...",
    "TotalCount": 2,
    "PageNumber": 1,
    "PageSize": 20,
    "Tasks": {
      "CDNTask": [
        {
          "TaskId": "1234567",
          "ObjectPath": "http://example.com/file.jpg",
          "Process": "100%",
          "Status": "Complete",
          "CreationTime": "2023-12-11T10:00:00Z",
          "ObjectType": "file"
        }
      ]
    }
  }
}
```

## Usage Example

```bash
# First, get a JWT token
JWT_TOKEN=$(cargo run -- generate-jwt --config config.toml --subject user123)

# Query refresh tasks
curl -X POST http://localhost:25150/api/aliyun/cdn/describeRefreshTasks \
  -H "Authorization: Bearer $JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "domain_name": "example.com",
    "object_type": "file",
    "status": "Complete",
    "page_number": 1,
    "page_size": 20
  }'
```

## Implementation Details

### Signature Algorithm (ACS3-HMAC-SHA256)

The implementation follows Alibaba Cloud's V3 signature specification:

1. **Canonical Request**: Constructs a canonical representation of the HTTP request including:
   - HTTP method
   - Request URI
   - Sorted query parameters
   - Sorted headers
   - SHA256 hash of request body

2. **String to Sign**: Creates the string to be signed:
   ```
   ACS3-HMAC-SHA256
   <SHA256 hash of canonical request>
   ```

3. **Signature**: Calculates HMAC-SHA256 of the string to sign using the AccessKeySecret

4. **Authorization Header**: Formats the authorization header:
   ```
   ACS3-HMAC-SHA256 Credential=<AccessKeyId>,SignedHeaders=<headers>,Signature=<signature>
   ```

### Required Headers

All API requests include these headers:
- `host`: API endpoint (e.g., "cdn.aliyuncs.com")
- `x-acs-action`: API action name (e.g., "DescribeRefreshTasks")
- `x-acs-version`: API version (e.g., "2018-05-10")
- `x-acs-date`: Request timestamp in ISO 8601 format
- `x-acs-signature-nonce`: Random UUID v4 for replay attack prevention
- `x-acs-content-sha256`: SHA256 hash of request body
- `authorization`: The signature authorization header

## Extending the Implementation

To add more Alibaba Cloud APIs:

1. Add new API methods to `CdnClient` in `src/aliyun/cdn.rs`, or create new client types for other services
2. Define request/response types with proper serde attributes
3. Create handler functions in `src/routes/aliyun_handlers.rs`
4. Register routes in `src/routes/mod.rs`

The signature algorithm in `src/aliyun/signature.rs` is generic and can be reused for any Alibaba Cloud V3 API.

## Testing

Unit tests for the signature algorithm are included in `src/aliyun/signature.rs`:

```bash
cargo test aliyun::signature
```

## References

- [Alibaba Cloud SDK: Request syntax and signature method V3](https://www.alibabacloud.com/help/en/sdk/product-overview/v3-request-structure-and-signature)
- [CDN DescribeRefreshTasks API](https://www.alibabacloud.com/help/en/cdn/developer-reference/api-cdn-2018-05-10-describerefreshtasks)
