use chrono::Utc;
use sysinfo::System;
use std::{fs::OpenOptions, io::Write, sync::Arc, time::Duration};
use tokio::time;
use crate::server::database::Database;

pub async fn start_performance_logger(db: Arc<Database>, log_path: &str) {
    let mut system = System::new_all();
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .expect("Unable to open performance log file");

    // Write header if file is empty
    if file.metadata().map(|m| m.len()).unwrap_or(0) == 0 {
        writeln!(file, "# Ruggine Server Performance Log").ok();
        writeln!(file, "# Timestamp, Active_Users, Groups, Total_Messages, CPU_Usage").ok();
    }

    loop {
        system.refresh_all();
        let cpu_usage = system.cpus().iter().map(|c| c.cpu_usage()).sum::<f32>() / system.cpus().len() as f32;
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S UTC");

        // Query DB for stats
        let active_users = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users WHERE is_online = 1")
            .fetch_one(&db.pool).await.unwrap_or(0);
        let groups = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM groups")
            .fetch_one(&db.pool).await.unwrap_or(0);
        let total_messages = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM encrypted_messages")
            .fetch_one(&db.pool).await.unwrap_or(0);

        writeln!(file, "{}, {}, {}, {}, {:.1}%", timestamp, active_users, groups, total_messages, cpu_usage).ok();
        file.flush().ok();
        time::sleep(Duration::from_secs(120)).await;
    }
}
