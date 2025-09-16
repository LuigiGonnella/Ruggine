<p align="center">
	<img src="./img/ruggineImage.png" alt="Ruggine logo" width="420" />
</p>

# Ruggine — Chat client/server

Ruggine is a modern, end-to-end messaging platform designed to be secure, modular, and production-ready. The server code is written in Rust using Tokio and SQLx; the desktop client uses Iced for the interface.

This guide provides clear and operational instructions on how to configure, build, deploy, and manage Ruggine in production.

## Table of Contents
- Overview
- Production Requirements
- Configuration and Secret Management
- Build, Containerization and Deploy
- Operations, Logging and Monitoring
- Security and Cryptographic Key Management
- Backup, Migrations and Disaster Recovery
- Scaling and Production Architecture
- Troubleshooting and FAQ
- Contributing

## Overview
Ruggine handles private and group chats with real-time messaging. Conversations are stored in the database in encrypted form (AES-256-GCM) and the server maintains a session and presence model for connected clients.

### New Features v2.0
- **WebSocket + Redis**: Real-time messaging that replaces database polling
- **Improved Scalability**: Support for multiple server instances via Redis pub/sub
- **Reduced Latency**: Instant messages instead of polling wait
- **Network Efficiency**: Only necessary messages instead of periodic queries

The project is designed to be easily integrated into CI/CD pipelines and containerized infrastructures.

## Production Requirements
- Toolchain: use stable Rust (compile in CI). Lock dependencies with `Cargo.lock`.
- Database: PostgreSQL 14+ (recommended); SQLite is for development only.
- Redis: Redis 6+ for WebSocket pub/sub and caching (mandatory for real-time messaging).
- TLS: valid certificates for ingress/endpoints. Using rustls or a reverse-proxy (nginx/traefik) is recommended.
- Secret management: Vault, AWS Secrets Manager, Azure Key Vault or equivalent for `ENCRYPTION_MASTER_KEY` and DB credentials.

## Configuration and Secret Management
Main parameters are managed through environment variables (or secret mounts). Minimal example:

```powershell
DATABASE_URL=postgres://ruggine_user:securepassword@postgres:5432/ruggine
REDIS_URL=redis://redis:6379
SERVER_HOST=0.0.0.0
SERVER_PORT=8443
WEBSOCKET_PORT=8444
ENABLE_ENCRYPTION=true
ENCRYPTION_MASTER_KEY=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
TLS_CERT_PATH=/etc/ssl/certs/ruggine.crt
TLS_KEY_PATH=/etc/ssl/private/ruggine.key
LOG_LEVEL=info
```

Operational guidelines:
- Do not store keys or credentials in the repository or in unprotected images.
- Store `ENCRYPTION_MASTER_KEY` in the platform's secret manager; load it at application boot.
- When rotating `ENCRYPTION_MASTER_KEY`, ensure you have procedures for migration or to maintain legacy keys to decrypt historical messages (see `doc/ENCRYPTION.md`).

## Build, Containerization and Deploy
It is recommended to build binaries in a dedicated CI job and distribute immutable Docker images.

- CI build example:

```powershell
cargo build --release --locked
```

- Example Dockerfile (multi-stage):

```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --locked

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/ruggine-server /usr/local/bin/ruggine-server
EXPOSE 8443
ENTRYPOINT ["/usr/local/bin/ruggine-server"]
```

- Recommended deployment:
	- For PoC: `docker-compose` with Postgres and TLS reverse-proxy.
	- For production: Kubernetes with Deployment, Service, Ingress, and Secret for `ENCRYPTION_MASTER_KEY`.

## Operations, Logging and Monitoring
- Logging: use structured format (JSON) and centralize. `LOG_LEVEL` manages verbosity level.
- Metrics: expose Prometheus-compatible metrics (latency, message_count, decryption_errors, active_connections).
- Health checks: implement `/healthz` and `/readyz` for orchestrator probes.
- Backup: perform regular DB backups and test restoration. Automate snapshots and retention policy.

## Security and Cryptographic Key Management
- Encryption: AES-256-GCM for message payloads. Messages are stored as JSON with `nonce`, `ciphertext` and metadata.
- Key protection: keep `ENCRYPTION_MASTER_KEY` in a vault. Access must be restricted and auditable.
- Key rotation: design a strategy (rolling re-encrypt, legacy key maintenance). Technical documentation in `doc/ENCRYPTION.md`.

## Backup, Migrations and Disaster Recovery
- Migrations: keep migration files versioned and apply them in CI with schema control.
- Recovery plan: script the steps for DB restore, `ENCRYPTION_MASTER_KEY` import and encrypted entity integrity verification.

## Scaling and Production Architecture
- Server: stateless, horizontally scalable behind LB.
- Database: PostgreSQL with replica and backup; consider partitioning for massive datasets.
- Recommendations: caching layer (Redis) for frequently accessed metadata and rate-limiting on ingress.

## Troubleshooting and FAQ
- Q: "Messages don't decrypt after restart" — A: verify `ENCRYPTION_MASTER_KEY` and look in log for entries with tag `[DECRYPTION FAILED]`.
- Q: "Registration failed with username already in use" — A: server returns user-friendly error `ERR: Username already in use`.
- Q: "Duplicate messages or presence loss" — A: check polling/ack process in `ChatService` and network probes.

## CI / Suggested Tests
- Unit tests: key derivation, encrypt/decrypt, and cryptographic helpers.
- Integration tests: CI job that runs temporary Postgres, applies migrations and simulates chat flows.

## Contributing
- Branching: feature/*, fix/*, release/*.
- PR: include description, tests and verification steps.

## License and Contacts
- Maintainers: Luigi Gonnella & Dorotea Monaco — open issues or PRs in the repository for technical questions.

---