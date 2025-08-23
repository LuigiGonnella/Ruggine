use ruggine_modulare::client::services::chat_service::ChatService;
use ruggine_modulare::server::config::ClientConfig;
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = ClientConfig::from_env();
    let host = format!("{}:{}", cfg.default_host, cfg.default_port);
    println!("Using host {}", host);

    let svc = Arc::new(Mutex::new(ChatService::new()));

    // login
    {
        let mut guard = svc.lock().await;
        let resp = guard.send_command(&host, "/login ciao ciaone".to_string()).await?;
        println!("LOGIN1 -> {}", resp);
        // extract token
        let token = resp.lines().find_map(|l| l.split("SESSION:").nth(1).map(|s| s.trim().to_string()));
        if token.is_none() {
            println!("No session token in login response");
            return Ok(());
        }
        let token = token.unwrap();

        // logout
        let resp = guard.send_command(&host, format!("/logout {}", token)).await?;
        println!("LOGOUT -> {}", resp);
    }

    // After logout, send login again (should reconnect)
    {
        let mut guard = svc.lock().await;
        let resp = guard.send_command(&host, "/login ciao ciaone".to_string()).await?;
        println!("LOGIN2 -> {}", resp);
    }

    Ok(())
}
