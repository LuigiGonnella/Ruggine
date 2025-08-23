// Modulo di parsing messaggi lato client
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
