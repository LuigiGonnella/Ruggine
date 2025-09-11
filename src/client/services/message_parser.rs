// Modulo di parsing messaggi lato client
use crate::client::models::app_state::ChatMessage;
use crate::common::crypto::CryptoManager;
use base64::{Engine as _, engine::general_purpose};

/// Attempt to decrypt a message content if it appears to be encrypted JSON
fn try_decrypt_content(content: &str, participants: &[String]) -> String {
    // Check if content looks like encrypted JSON
    if content.contains("ciphertext") && content.contains("nonce") {
        if let Ok(encrypted_data) = serde_json::from_str::<serde_json::Value>(content) {
            if let Some(master_key) = CryptoManager::load_master_key_from_env() {
                let chat_key = CryptoManager::generate_chat_key(participants, &master_key);
                
                if let (Some(ciphertext), Some(nonce)) = (
                    encrypted_data.get("ciphertext").and_then(|v| v.as_str()),
                    encrypted_data.get("nonce").and_then(|v| v.as_str())
                ) {
                    // Decode base64 encoded ciphertext and nonce
                    if let (Ok(cipher_bytes), Ok(nonce_bytes)) = (
                        general_purpose::STANDARD.decode(ciphertext),
                        general_purpose::STANDARD.decode(nonce)
                    ) {
                        if let Ok(decrypted) = CryptoManager::decrypt_message(&cipher_bytes, &nonce_bytes, &chat_key) {
                            return decrypted;
                        }
                    }
                }
            }
        }
    }
    
    // If decryption fails or content is not encrypted, return as-is
    content.to_string()
}

/// Parse server `OK: Messages:\n<lines...>` responses into Vec<String>.
pub fn parse_messages(resp: &str) -> Result<Vec<String>, &'static str> {
	let trimmed = resp.trim();
	if !trimmed.starts_with("OK: Messages:") {
		return Err("unexpected response format");
	}
	// Split after the first newline and collect remaining non-empty lines
	let mut parts = trimmed.splitn(2, '\n');
	parts.next(); // skip the OK header
	if let Some(body) = parts.next() {
		let msgs: Vec<String> = body.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect();
		Ok(msgs)
	} else {
		Ok(vec![])
	}
}

/// Parse private messages from server response into ChatMessage structs with decryption
pub fn parse_private_messages_with_participants(resp: &str, participants: &[String]) -> Result<Vec<ChatMessage>, &'static str> {
    let trimmed = resp.trim();
    if !trimmed.starts_with("OK: Messages:") {
        return Err("unexpected response format");
    }
    
    let mut parts = trimmed.splitn(2, '\n');
    parts.next(); // skip the OK header
    
    if let Some(body) = parts.next() {
        let mut messages = Vec::new();
        
        for line in body.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            
            // Expected format: [timestamp] sender: message
            if let Some(bracket_end) = line.find(']') {
                if line.starts_with('[') {
                    let timestamp_str = &line[1..bracket_end];
                    let rest = &line[bracket_end + 1..].trim();
                    
                    if let Some(colon_pos) = rest.find(':') {
                        let sender = rest[..colon_pos].trim().to_string();
                        let raw_content = rest[colon_pos + 1..].trim().to_string();
                        
                        if let Ok(timestamp) = timestamp_str.parse::<i64>() {
                            let formatted_time = format_timestamp(timestamp);
                            
                            // Try to decrypt the content if it's encrypted
                            let decrypted_content = try_decrypt_content(&raw_content, participants);
                            
                            messages.push(ChatMessage {
                                sender,
                                content: decrypted_content,
                                timestamp,
                                formatted_time,
                                sent_at: timestamp,
                            });
                        }
                    }
                }
            }
        }
        
        // Sort by timestamp to ensure chronological order
        messages.sort_by_key(|m| m.timestamp);
        Ok(messages)
    } else {
        Ok(vec![])
    }
}

/// Parse private messages from server response into ChatMessage structs (legacy version)
pub fn parse_private_messages(resp: &str) -> Result<Vec<ChatMessage>, &'static str> {
    // Use empty participants list for backward compatibility
    parse_private_messages_with_participants(resp, &[])
}

pub fn format_timestamp(timestamp: i64) -> String {
    use chrono::{DateTime, Utc, Local, TimeZone};
    
    let dt = Utc.timestamp_opt(timestamp, 0).single().unwrap_or_else(Utc::now);
    let local_dt: DateTime<Local> = dt.with_timezone(&Local);
    
    // Format as HH:MM
    local_dt.format("%H:%M").to_string()
}

/// Parse group messages from server response into ChatMessage structs with decryption
pub fn parse_group_messages_with_participants(resp: &str, participants: &[String]) -> Result<Vec<ChatMessage>, &'static str> {
    let trimmed = resp.trim();
    if !trimmed.starts_with("OK: Messages:") {
        return Err("unexpected response format");
    }
    
    let mut parts = trimmed.splitn(2, '\n');
    parts.next(); // skip the OK header
    
    if let Some(body) = parts.next() {
        let mut messages = Vec::new();
        
        for line in body.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            
            // Expected format: [timestamp] sender_name: message
            if let Some(bracket_end) = line.find(']') {
                if line.starts_with('[') {
                    let timestamp_str = &line[1..bracket_end];
                    let rest = &line[bracket_end + 1..].trim();
                    
                    if let Some(colon_pos) = rest.find(':') {
                        let sender_name = rest[..colon_pos].trim().to_string();
                        let raw_content = rest[colon_pos + 1..].trim().to_string();
                        
                        if let Ok(timestamp) = timestamp_str.parse::<i64>() {
                            let formatted_time = format_timestamp(timestamp);
                            
                            // Try to decrypt the content if it's encrypted
                            let decrypted_content = try_decrypt_content(&raw_content, participants);
                            
                            messages.push(ChatMessage {
                                sender: sender_name, // Now shows actual username
                                content: decrypted_content,
                                timestamp,
                                formatted_time,
                                sent_at: timestamp,
                            });
                        }
                    }
                }
            }
        }
        
        // Sort by timestamp to ensure chronological order
        messages.sort_by_key(|m| m.timestamp);
        Ok(messages)
    } else {
        Ok(vec![])
    }
}

/// Parse group messages from server response into ChatMessage structs (legacy version)
pub fn parse_group_messages(resp: &str) -> Result<Vec<ChatMessage>, &'static str> {
    // Use empty participants list for backward compatibility
    parse_group_messages_with_participants(resp, &[])
}
