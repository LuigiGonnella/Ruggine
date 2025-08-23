// Modulo di gestione amicizie lato client e messaggistica privata
use crate::client::services::chat_service::ChatService;
use crate::client::services::message_parser;
use std::sync::Arc;
use tokio::sync::Mutex;

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
		let cmd = format!("/send_private {} {} {}", session_token, to_username, message);
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
}
