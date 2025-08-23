// Modulo di gestione gruppi lato client
use crate::client::services::chat_service::ChatService;
use crate::client::services::message_parser;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Default)]
pub struct GroupService;

impl GroupService {
	pub fn new() -> Self { Self {} }

	/// Create a new group with `group_name`.
	pub async fn create_group(
		svc: &Arc<Mutex<ChatService>>,
		host: &str,
		session_token: &str,
		group_name: &str,
	) -> anyhow::Result<String> {
		let mut guard = svc.lock().await;
		let cmd = format!("/create_group {} {}", session_token, group_name);
		let resp = guard.send_command(host, cmd).await?;
		Ok(resp)
	}

	/// List groups for the current user, returns Vec of "id:name" strings.
	pub async fn my_groups(
		svc: &Arc<Mutex<ChatService>>,
		host: &str,
		session_token: &str,
	) -> anyhow::Result<Vec<String>> {
		let mut guard = svc.lock().await;
		let cmd = format!("/my_groups {}", session_token);
		let resp = guard.send_command(host, cmd).await?;
		if !resp.starts_with("OK:") {
			return Err(anyhow::anyhow!(resp));
		}
		// expected: "OK: My groups: id:name, id2:name2"
		if let Some(after) = resp.splitn(2, ':').nth(2) {
			let list = after.trim().split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
			Ok(list)
		} else {
			Ok(vec![])
		}
	}

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
		// server expects /send_group_message
		let cmd = format!("/send_group_message {} {} {}", session_token, group_name, message);
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

	/// Invite a user to a group. args: <session> <group_id> <username>
	pub async fn invite(
		svc: &Arc<Mutex<ChatService>>,
		host: &str,
		session_token: &str,
		group_id: &str,
		username: &str,
	) -> anyhow::Result<String> {
		let mut guard = svc.lock().await;
		let cmd = format!("/invite_to_group {} {} {}", session_token, group_id, username);
		let resp = guard.send_command(host, cmd).await?;
		Ok(resp)
	}

	/// Accept a group invite. args: <session> <invite_id>
	pub async fn accept_invite(
		svc: &Arc<Mutex<ChatService>>,
		host: &str,
		session_token: &str,
		invite_id: &str,
	) -> anyhow::Result<String> {
		let mut guard = svc.lock().await;
		let cmd = format!("/accept_group_invite {} {}", session_token, invite_id);
		let resp = guard.send_command(host, cmd).await?;
		Ok(resp)
	}

	/// Reject a group invite. args: <session> <invite_id>
	pub async fn reject_invite(
		svc: &Arc<Mutex<ChatService>>,
		host: &str,
		session_token: &str,
		invite_id: &str,
	) -> anyhow::Result<String> {
		let mut guard = svc.lock().await;
		let cmd = format!("/reject_group_invite {} {}", session_token, invite_id);
		let resp = guard.send_command(host, cmd).await?;
		Ok(resp)
	}

	/// List pending invites for current user. args: <session>
	pub async fn my_invites(
		svc: &Arc<Mutex<ChatService>>,
		host: &str,
		session_token: &str,
	) -> anyhow::Result<Vec<String>> {
		let mut guard = svc.lock().await;
		let cmd = format!("/my_group_invites {}", session_token);
		let resp = guard.send_command(host, cmd).await?;
		if !resp.starts_with("OK:") {
			return Err(anyhow::anyhow!(resp));
		}
		// expected: "OK: Invites: id:group_name from:username, ..." - fallback to splitting by commas
		let after = resp.splitn(2, ':').nth(2).unwrap_or("");
		let list = after.trim().split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
		Ok(list)
	}

	/// Join a group. args: <session> <group_id>
	pub async fn join_group(
		svc: &Arc<Mutex<ChatService>>,
		host: &str,
		session_token: &str,
		group_id: &str,
	) -> anyhow::Result<String> {
		let mut guard = svc.lock().await;
		let cmd = format!("/join_group {} {}", session_token, group_id);
		let resp = guard.send_command(host, cmd).await?;
		Ok(resp)
	}

	/// Leave a group. args: <session> <group_id>
	pub async fn leave_group(
		svc: &Arc<Mutex<ChatService>>,
		host: &str,
		session_token: &str,
		group_id: &str,
	) -> anyhow::Result<String> {
		let mut guard = svc.lock().await;
		let cmd = format!("/leave_group {} {}", session_token, group_id);
		let resp = guard.send_command(host, cmd).await?;
		Ok(resp)
	}
}
