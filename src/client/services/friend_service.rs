// Modulo di gestione amicizie lato client e messaggistica privata
use crate::client::services::chat_service::ChatService;
use crate::client::services::message_parser;
use std::sync::Arc;
use tokio::sync::Mutex;
use anyhow::Result;

#[derive(Debug, Default)]
pub struct FriendService;

impl FriendService {
	pub fn new() -> Self { Self {} }

	/// Send a private message to another user via ChatService.
	pub async fn send_private_message(
		svc: &Arc<Mutex<ChatService>>,
		host: &str,
		session_token: &str,
		to_username: &str,
		message: &str,
	) -> anyhow::Result<String> {
		let mut guard = svc.lock().await;
		// server expects /send_private_message
		let cmd = format!("/send_private_message {} {} {}", session_token, to_username, message);
		let resp = guard.send_command(host, cmd).await?;
		Ok(resp)
	}

	/// Retrieve private messages with another user.
	pub async fn get_private_messages(
		svc: &Arc<Mutex<ChatService>>,
		host: &str,
		session_token: &str,
		other_username: &str,
	) -> anyhow::Result<Vec<String>> {
		let mut guard = svc.lock().await;
		let cmd = format!("/get_private_messages {} {}", session_token, other_username);
		let resp = guard.send_command(host, cmd).await?;
		let msgs = message_parser::parse_messages(&resp).map_err(|e| anyhow::anyhow!(e))?;
		Ok(msgs)
	}

	/// Delete private messages with another user.
	pub async fn delete_private_messages(
		svc: &Arc<Mutex<ChatService>>,
		host: &str,
		session_token: &str,
		other_username: &str,
	) -> anyhow::Result<String> {
		let mut guard = svc.lock().await;
		let cmd = format!("/delete_private_messages {} {}", session_token, other_username);
		let resp = guard.send_command(host, cmd).await?;
		Ok(resp)
	}

	/// Send a friend request to another user
	pub async fn send_friend_request(
		svc: &Arc<Mutex<ChatService>>,
		host: &str,
		session_token: &str,
		to_username: &str,
		message: &str,
	) -> Result<String> {
		let mut guard = svc.lock().await;
		let cmd = format!("/send_friend_request {} {} {}", session_token, to_username, message);
		let resp = guard.send_command(host, cmd).await?;
		Ok(resp)
	}

	/// Accept a pending friend request from `from_username`
	pub async fn accept_friend_request(
		svc: &Arc<Mutex<ChatService>>,
		host: &str,
		session_token: &str,
		from_username: &str,
	) -> Result<String> {
		let mut guard = svc.lock().await;
		let cmd = format!("/accept_friend_request {} {}", session_token, from_username);
		let resp = guard.send_command(host, cmd).await?;
		Ok(resp)
	}

	/// Reject a pending friend request from `from_username`
	pub async fn reject_friend_request(
		svc: &Arc<Mutex<ChatService>>,
		host: &str,
		session_token: &str,
		from_username: &str,
	) -> Result<String> {
		let mut guard = svc.lock().await;
		let cmd = format!("/reject_friend_request {} {}", session_token, from_username);
		let resp = guard.send_command(host, cmd).await?;
		Ok(resp)
	}

	/// List friends for current user. Returns a Vec of usernames on success.
	pub async fn list_friends(
		svc: &Arc<Mutex<ChatService>>,
		host: &str,
		session_token: &str,
	) -> Result<Vec<String>> {
		let mut guard = svc.lock().await;
		let cmd = format!("/list_friends {}", session_token);
		let resp = guard.send_command(host, cmd).await?;
		if !resp.starts_with("OK:") {
			return Err(anyhow::anyhow!(resp));
		}
		// Expected format: "OK: Friends: alice, bob"
		if let Some(after) = resp.splitn(2, ':').nth(2) {
			let list = after.trim().split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
			Ok(list)
		} else {
			Ok(vec![])
		}
	}

	/// Retrieve received (incoming) friend requests. Returns Vec of "username: message" entries.
	pub async fn received_friend_requests(
		svc: &Arc<Mutex<ChatService>>,
		host: &str,
		session_token: &str,
	) -> Result<Vec<String>> {
		let mut guard = svc.lock().await;
		let cmd = format!("/received_friend_requests {}", session_token);
		let resp = guard.send_command(host, cmd).await?;
		if !resp.starts_with("OK:") {
			return Err(anyhow::anyhow!(resp));
		}
		// Expected: "OK: Richieste ricevute: alice: msg | bob: msg"
		if let Some(after) = resp.splitn(2, ':').nth(2) {
			// split by '|' then trim
			let items = after.split('|').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
			Ok(items)
		} else {
			Ok(vec![])
		}
	}

	/// Retrieve sent (outgoing) friend requests. Returns Vec of "username: message" entries.
	pub async fn sent_friend_requests(
		svc: &Arc<Mutex<ChatService>>,
		host: &str,
		session_token: &str,
	) -> Result<Vec<String>> {
		let mut guard = svc.lock().await;
		let cmd = format!("/sent_friend_requests {}", session_token);
		let resp = guard.send_command(host, cmd).await?;
		if !resp.starts_with("OK:") {
			return Err(anyhow::anyhow!(resp));
		}
		if let Some(after) = resp.splitn(2, ':').nth(2) {
			let items = after.split('|').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
			Ok(items)
		} else {
			Ok(vec![])
		}
	}
}
