// Integration test demonstrating Alibaba Cloud signature generation
// This test verifies that the signature algorithm produces valid output
// without requiring actual API credentials

use janus::aliyun::signature::AliyunSigner;
use std::collections::BTreeMap;

#[test]
fn test_signature_generation() {
    // Create a signer with test credentials
    let signer = AliyunSigner::new(
        "test_access_key_id".to_string(),
        "test_access_key_secret".to_string(),
    );

    // Prepare test request parameters
    let method = "POST";
    let uri = "/";
    let query_params = BTreeMap::new();

    let mut headers = BTreeMap::new();
    headers.insert("host".to_string(), "cdn.aliyuncs.com".to_string());
    headers.insert(
        "x-acs-action".to_string(),
        "DescribeRefreshTasks".to_string(),
    );
    headers.insert("x-acs-version".to_string(), "2018-05-10".to_string());

    let body = r#"{"PageNumber":1,"PageSize":20}"#;

    // Generate signature
    let (authorization, date, nonce, content_sha256) =
        signer.sign_request(method, uri, &query_params, headers, body);

    // Verify the signature components are generated
    assert!(authorization.starts_with("ACS3-HMAC-SHA256"));
    assert!(authorization.contains("Credential=test_access_key_id"));
    assert!(authorization.contains("SignedHeaders="));
    assert!(authorization.contains("Signature="));

    // Verify date format (ISO 8601)
    assert!(date.contains('T'));
    assert!(date.ends_with('Z'));

    // Verify nonce is a valid UUID
    assert_eq!(nonce.len(), 36); // UUID v4 format: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
    assert_eq!(nonce.chars().filter(|&c| c == '-').count(), 4);

    // Verify content hash is 64 hex characters (SHA256)
    assert_eq!(content_sha256.len(), 64);
    assert!(content_sha256.chars().all(|c| c.is_ascii_hexdigit()));

    println!("✓ Signature generation successful!");
    println!("  Authorization: {}", authorization);
    println!("  Date: {}", date);
    println!("  Nonce: {}", nonce);
    println!("  Content-SHA256: {}", content_sha256);
}

#[test]
fn test_cdn_client_creation() {
    use janus::aliyun::CdnClient;

    // Test that we can create a CDN client
    let _client = CdnClient::new(
        "test_key_id".to_string(),
        "test_secret".to_string(),
        "cdn.aliyuncs.com".to_string(),
    );

    // Just verify it compiles and creates successfully
    println!("✓ CDN client created successfully");
}

#[test]
fn test_request_params_serialization() {
    use janus::aliyun::cdn::DescribeRefreshTasksParams;

    // Test that params can be created and serialized
    let params = DescribeRefreshTasksParams {
        task_id: Some("123456".to_string()),
        object_path: Some("http://example.com/file.jpg".to_string()),
        page_number: Some(1),
        object_type: Some("file".to_string()),
        domain_name: Some("example.com".to_string()),
        status: Some("Complete".to_string()),
        page_size: Some(20),
        start_time: Some("2023-12-10T08:00:00Z".to_string()),
        end_time: Some("2023-12-12T08:00:00Z".to_string()),
        resource_group_id: None,
    };

    // Serialize to JSON
    let json = serde_json::to_string(&params).expect("Failed to serialize params");

    // Verify serialization works
    assert!(json.contains("TaskId"));
    assert!(json.contains("123456"));
    assert!(json.contains("ObjectPath"));
    assert!(json.contains("example.com"));

    println!("✓ Request params serialization successful");
    println!("  JSON: {}", json);
}
