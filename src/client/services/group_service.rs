// Modulo di gestione gruppi lato client
use crate::client::services::chat_service::ChatService;
use crate::client::services::message_parser;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Default)]
pub struct GroupService;

impl GroupService {
	pub fn new() -> Self { Self {} }

	/// Send a group chat message via the shared ChatService.
	/// `host` must be a host:port string (eg. "127.0.0.1:5000").
	pub async fn send_group_message(
		svc: &Arc<Mutex<ChatService>>,
		host: &str,
		session_token: &str,
		group_name: &str,
		message: &str,
	) -> anyhow::Result<String> {
		let mut guard = svc.lock().await;
		let cmd = format!("/send_group {} {} {}", session_token, group_name, message);
		let resp = guard.send_command(host, cmd).await?;
		Ok(resp)
	}

	/// Retrieve messages for a group and return them as a vector of lines.
	pub async fn get_group_messages(
		svc: &Arc<Mutex<ChatService>>,
		host: &str,
		session_token: &str,
		group_name: &str,
	) -> anyhow::Result<Vec<String>> {
		let mut guard = svc.lock().await;
		let cmd = format!("/get_group_messages {} {}", session_token, group_name);
		let resp = guard.send_command(host, cmd).await?;
		let msgs = message_parser::parse_messages(&resp).map_err(|e| anyhow::anyhow!(e))?;
		Ok(msgs)
	}

	/// Delete all messages in a group (requires membership).
	pub async fn delete_group_messages(
		svc: &Arc<Mutex<ChatService>>,
		host: &str,
		session_token: &str,
		group_id: &str,
	) -> anyhow::Result<String> {
		let mut guard = svc.lock().await;
		let cmd = format!("/delete_group_messages {} {}", session_token, group_id);
		let resp = guard.send_command(host, cmd).await?;
		Ok(resp)
	}
}
