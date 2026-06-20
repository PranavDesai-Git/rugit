use std::io::{Read, Write};
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue};

// Helper to format strings into Git's pkt-line format
fn encode_pkt_line(payload: &str) -> Vec<u8> {
    let len = payload.len() + 4;
    let hex_len = format!("{:04x}", len);
    let mut result = hex_len.into_bytes();
    result.extend_from_slice(payload.as_bytes());
    result
}

/// Resolves user credentials dynamically via Environment Variables or Terminal Prompt
fn resolve_credentials() -> (String, String) {
    // 1. Check for standard environment variables first
    if let (Ok(user), Ok(token)) = (std::env::var("RUGIT_USER"), std::env::var("RUGIT_TOKEN")) {
        return (user, token);
    }
    // Backup check for standard GitHub environment variables
    if let (Ok(user), Ok(token)) = (std::env::var("GITHUB_USER"), std::env::var("GITHUB_TOKEN")) {
        return (user, token);
    }

    // 2. Fallback to interactive terminal input if variables don't exist
    println!("💡 Hint: Set RUGIT_USER and RUGIT_TOKEN env variables to skip this prompt.");
    
    print!("Enter GitHub Username: ");
    std::io::stdout().flush().unwrap();
    let mut username = String::new();
    std::io::stdin().read_line(&mut username).expect("Failed to read username");

    print!("Enter GitHub Personal Access Token (PAT): ");
    std::io::stdout().flush().unwrap();
    let mut token = String::new();
    std::io::stdin().read_line(&mut token).expect("Failed to read token");

    // Clean up trailing newlines from terminal input (\r\n on Windows or \n on Linux)
    (username.trim().to_string(), token.trim().to_string())
}

/// Pushes the custom repository trinity directly to a remote GitHub repository over HTTPS.
pub fn test_push_connection(remote_url: &str) {
    // Dynamically fetch credentials based on who is running the tool
    let (username, github_token) = resolve_credentials();

    if username.is_empty() || github_token.is_empty() {
        panic!("Error: Username or Token cannot be empty!");
    }

    let client = Client::new();
    let base_url = remote_url.trim_end_matches('/');

    // =========================================================================
    // PHASE 1: DISCOVERY (GET Request)
    // =========================================================================
    let discovery_url = format!("{}/info/refs?service=git-receive-pack", base_url);
    println!("\n1. Probing remote GitHub endpoint via GET...");

    let res = client.get(&discovery_url)
        .basic_auth(&username, Some(&github_token))
        .send()
        .expect("Failed to execute discovery GET request");

    if !res.status().is_success() {
        panic!("GitHub authentication failed or repository not found! Status: {}", res.status());
    }

    println!("Successfully authenticated as '{}'! Server responded with 200 OK.\n", username);

    // =========================================================================
    // PHASE 2: PACKFILE GENERATION & RPC STREAM (POST Request)
    // =========================================================================
    println!("2. Generating the complete packfile trinity (Blob -> Tree -> Commit)...");
    let (pack_data, commit_hex_sha) = crate::pack::create_final_packfile();

    let mut post_body = Vec::new();
    let zero_oid = "0000000000000000000000000000000000000000";
    let branch = "refs/heads/master";
    let update_msg = format!("{} {} {}\0report-status\n", zero_oid, commit_hex_sha, branch);
    
    post_body.extend_from_slice(&encode_pkt_line(&update_msg));
    post_body.extend_from_slice(b"0000"); 
    post_body.extend_from_slice(&pack_data); 

    let mut headers = HeaderMap::new();
    headers.insert(
        reqwest::header::CONTENT_TYPE,
        HeaderValue::from_static("application/x-git-receive-pack-request")
    );
    headers.insert(
        reqwest::header::ACCEPT,
        HeaderValue::from_static("application/x-git-receive-pack-result")
    );

    let rpc_url = format!("{}/git-receive-pack", base_url);
    println!("3. Uploading payload to GitHub via POST ({} bytes total)...", post_body.len());

    let mut response = client.post(&rpc_url)
        .headers(headers)
        .basic_auth(&username, Some(&github_token))
        .body(post_body)
        .send()
        .expect("Failed to execute RPC POST request");

    let mut final_response = String::new();
    response.read_to_string(&mut final_response).unwrap();
    
    println!("\n--- GitHub Server Response ---");
    println!("{}", final_response);
    println!("-------------------------------");
}