use std::io::{Read, Write};
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue};
use std::fs;

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
    if let (Ok(user), Ok(token)) = (std::env::var("RUGIT_USER"), std::env::var("RUGIT_TOKEN")) {
        return (user, token);
    }
    if let (Ok(user), Ok(token)) = (std::env::var("GITHUB_USER"), std::env::var("GITHUB_TOKEN")) {
        return (user, token);
    }

    println!("Hint: Set RUGIT_USER and RUGIT_TOKEN env variables to skip this prompt.");
    
    print!("Enter GitHub Username: ");
    std::io::stdout().flush().unwrap();
    let mut username = String::new();
    std::io::stdin().read_line(&mut username).expect("Failed to read username");

    print!("Enter GitHub Personal Access Token (PAT): ");
    std::io::stdout().flush().unwrap();
    let mut token = String::new();
    std::io::stdin().read_line(&mut token).expect("Failed to read token");

    (username.trim().to_string(), token.trim().to_string())
}

fn get_local_head_commit() -> Option<[u8; 20]> {
    if let Ok(head_content) = fs::read_to_string(".git/HEAD") {
        let ref_path = head_content.trim().trim_start_matches("ref: ").to_string();
        let full_ref_path = format!(".git/{}", ref_path);
        if let Ok(commit_hex) = fs::read_to_string(full_ref_path) {
            let mut sha1 = [0u8; 20];
            if hex::decode_to_slice(commit_hex.trim(), &mut sha1).is_ok() {
                return Some(sha1);
            }
        }
    }
    None
}

fn parse_remote_branch_sha(discovery_body: &str, target_branch: &str) -> String {
    let zero_oid = "0000000000000000000000000000000000000000".to_string();
    
    for line in discovery_body.lines() {
        if line.contains('#') || line.is_empty() {
            continue;
        }

        let clean_line = line.trim().split('\0').next().unwrap_or(line);
        let parts: Vec<&str> = clean_line.split_whitespace().collect();
        
        if parts.len() >= 2 {
            let ref_name = parts[1];
            if ref_name == target_branch {
                let sha = parts[0];
                if sha.len() >= 40 {
                    return sha[sha.len() - 40..].to_string();
                }
            }
        }
    }
    zero_oid
}

pub fn test_push_connection(remote_url: &str) {
    let (username, github_token) = resolve_credentials();

    if username.is_empty() || github_token.is_empty() {
        panic!("Error: Username or Token cannot be empty!");
    }

    let local_commit_sha1 = get_local_head_commit().expect(
        "Error: Could not read local HEAD. Make sure you have committed something using base::commit first!"
    );

    let client = Client::new();
    let base_url = remote_url.trim_end_matches('/');
    let target_branch = "refs/heads/main";

    let discovery_url = format!("{}/info/refs?service=git-receive-pack", base_url);
    println!("\n1. Probing remote GitHub endpoint via GET...");

    let mut res = client.get(&discovery_url)
        .basic_auth(&username, Some(&github_token))
        .send()
        .expect("Failed to execute discovery GET request");

    if !res.status().is_success() {
        panic!("GitHub authentication failed or repository not found! Status: {}", res.status());
    }

    let mut discovery_body = String::new();
    res.read_to_string(&mut discovery_body).unwrap();
    println!("Successfully authenticated as '{}'! Received remote ref listings.", username);

    let remote_old_oid = parse_remote_branch_sha(&discovery_body, target_branch);
    println!("Remote target state for {} is currently: {}", target_branch, remote_old_oid);

    println!("\n2. Dynamically tracing and gathering packfile objects from local database...");
    
    let (pack_data, local_commit_hex) = crate::pack::create_dynamic_packfile(&local_commit_sha1)
        .expect("Failed to dynamically traverse and generate packfile");

    let mut post_body = Vec::new();
    let update_msg = format!("{} {} {}\0report-status\n", remote_old_oid, local_commit_hex, target_branch);
    
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
    println!("3. Uploading real repository history via POST ({} bytes total)...", post_body.len());

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
