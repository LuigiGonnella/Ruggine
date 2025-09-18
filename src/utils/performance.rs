use chrono::Utc;
use sysinfo::System;
use std::{fs::OpenOptions, io::Write, sync::Arc, time::Duration};
use tokio::time;
use crate::server::database::Database;
use log::{info, error, warn};

pub async fn start_performance_logger(db: Arc<Database>, log_path: &str) {
    let mut system = System::new_all();
    
    // Try to create/open the log file
    let mut file = match OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path) {
        Ok(f) => f,
        Err(e) => {
            error!("Unable to open performance log file '{}': {}", log_path, e);
            return;
        }
    };
    

    // Write header if file is empty
    if file.metadata().map(|m| m.len()).unwrap_or(0) == 0 {
        if let Err(e) = writeln!(file, "# Ruggine Server Performance Log") {
            error!("Failed to write header to performance log: {}", e);
            return;
        }
        if let Err(e) = writeln!(file, "# Timestamp, Active_Users, Groups, Total_Messages, CPU_Usage") {
            error!("Failed to write header to performance log: {}", e);
            return;
        }
        info!("📊 Performance log initialized: {}", log_path);
    }

    loop {
        system.refresh_all();
        let cpu_usage = system.cpus().iter().map(|c| c.cpu_usage()).sum::<f32>() / system.cpus().len() as f32;
        // Use local time (UTC+2 for Italy) instead of UTC for presentation
        let local_time = Utc::now() + chrono::Duration::hours(2);
        let timestamp = local_time.format("%Y-%m-%d %H:%M:%S UTC");

        // Query DB for stats with error handling
        let active_users = match sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users WHERE is_online = 1")
            .fetch_one(&db.pool).await {
            Ok(count) => count,
            Err(e) => {
                warn!("Failed to query active users: {}", e);
                -1
            }
        };
        
        let groups = match sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM groups")
            .fetch_one(&db.pool).await {
            Ok(count) => count,
            Err(e) => {
                warn!("Failed to query groups: {}", e);
                -1
            }
        };
        
        let total_messages = match sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM encrypted_messages")
            .fetch_one(&db.pool).await {
            Ok(count) => count,
            Err(e) => {
                warn!("Failed to query messages: {}", e);
                -1
            }
        };

        // Log to console
        info!("📊 Performance - Active Users: {}, Groups: {}, Messages: {}, CPU: {:.1}%", 
            active_users, groups, total_messages, cpu_usage);

        // Write to file
        if let Err(e) = writeln!(file, "{}, {}, {}, {}, {:.1}%", timestamp, active_users, groups, total_messages, cpu_usage) {
            error!("Failed to write to performance log: {}", e);
        } else if let Err(e) = file.flush() {
            error!("Failed to flush performance log: {}", e);
        }
        
        time::sleep(Duration::from_secs(120)).await;
    }
}
