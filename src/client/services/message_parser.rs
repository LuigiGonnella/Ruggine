// Modulo di parsing messaggi lato client
use crate::client::models::app_state::ChatMessage;

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

/// Parse private messages from server response into ChatMessage structs
pub fn parse_private_messages(resp: &str) -> Result<Vec<ChatMessage>, &'static str> {
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
                        let content = rest[colon_pos + 1..].trim().to_string();
                        
                        if let Ok(timestamp) = timestamp_str.parse::<i64>() {
                            let formatted_time = format_timestamp(timestamp);
                            
                            messages.push(ChatMessage {
                                sender,
                                content,
                                timestamp,
                                formatted_time,
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

fn format_timestamp(timestamp: i64) -> String {
    use chrono::{DateTime, Utc, Local, TimeZone};
    
    let dt = Utc.timestamp_opt(timestamp, 0).single().unwrap_or_else(|| Utc::now());
    let local_dt: DateTime<Local> = dt.with_timezone(&Local);
    
    // Format as HH:MM
    local_dt.format("%H:%M").to_string()
}
