Deploy guide per Ruggine (production-ready)

Scopo: fornire una procedura passo-passo per mettere in produzione il server Ruggine, incluse le scelte di DB, TLS, containerizzazione e operazioni minime.

1) Scelte architetturali consigliate
- Backend: eseguire `ruggine-server` in un container o come servizio systemd.
- DB: Postgres raccomandato per concorrenza e affidabilità. SQLite va bene solo per POC o singolo utente.
- TLS: terminare TLS al reverse-proxy (nginx/Caddy/Traefik) oppure direttamente con `ruggine-server` se preferisci (server supporta rustls e legge `TLS_CERT_PATH`/`TLS_KEY_PATH`).
- Secrets: non committare file PEM o credenziali; usare secret manager o montare volumi con permessi ristretti.

2) Variabili d'ambiente importanti
- `SERVER_HOST`, `SERVER_PORT` — dove bindare il server.
- `DATABASE_URL` — es: postgres://user:pass@host:5432/db
- `ENABLE_ENCRYPTION` — true/false; se true il server cercherà `TLS_CERT_PATH` e `TLS_KEY_PATH`.
- `TLS_CERT_PATH`, `TLS_KEY_PATH` — percorsi ai PEM.
- `SESSION_EXPIRY_DAYS` — durata della sessione (giorni).
- `LOG_LEVEL` — info/debug/error.

3) Preparare il database Postgres
- Creare un database e un utente con permessi:
  - CREATE USER ruggine WITH PASSWORD 'password';
  - CREATE DATABASE ruggine_db OWNER ruggine;
- Impostare `DATABASE_URL` nella env: `postgres://ruggine:password@postgres:5432/ruggine_db`
- Al primo avvio il server chiama `db.migrate()` e crea le tabelle automaticamente.

4) Certificati TLS
- Consigliato: usare Let’s Encrypt con reverse-proxy (Traefik/Caddy) o cert-manager su k8s.
- Se vuoi usare il server rustls direttamente: monta `cert.pem` e `key.pem` in `/app/certs` e imposta `TLS_CERT_PATH` e `TLS_KEY_PATH`.

5) Containerizzazione (esempio rapido)
- Crea un `Dockerfile` per il server Ruggine basato su immagine Rust
- Usa `docker-compose` per orchestrare Postgres, Redis e il server Ruggine
- Esempio base:
```dockerfile
FROM rust:1.70-slim as builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin ruggine-server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/ruggine-server /usr/local/bin/
CMD ["ruggine-server"]
```

6) Avvio in produzione (systemd example)
- Crea un service file per systemd:
```ini
[Unit]
Description=Ruggine Chat Server
After=network.target postgresql.service redis.service

[Service]
Type=simple
User=ruggine
WorkingDirectory=/opt/ruggine
ExecStart=/opt/ruggine/ruggine-server
EnvironmentFile=/etc/ruggine/server.env
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```
- Salva come `/etc/systemd/system/ruggine-server.service`
- Abilita con: `systemctl enable ruggine-server.service`

7) Healthchecks e monitoraggio
- Aggiungi un healthcheck esterno (script che invia `/help` o `validate_session` con un token di test) e integra nel sistema di monitoring.
- Log ruota con logrotate o usa stdout/stderr in container e fa log collection con ELK/Promtail.

8) Sicurezza client-side
- Client desktop: mantenere `keyring` abilitato. In produzione evita il fallback file; se vuoi farlo, aggiungi `KEYRING_FALLBACK=false` e modifica `session_store` per onfail->error.

9) Session rotation e gestione
- Per maggiore sicurezza: implementare refresh token con breve vita per l'access token.
- Se vuoi limitare dispositivi per utente, implementa logica in `login` per rimuovere sessioni più vecchie o rifiutare nuove oltre il limite.

10) Rollout
- Test in staging con Postgres e TLS reali.
- Migrare il traffico dietro reverse-proxy e abilitare metriche.

Appendice: comandi utili
- Build release:
  - cargo build --release
- Avviare server:
  - SERVER_HOST=0.0.0.0 SERVER_PORT=5000 DATABASE_URL=postgres://... ENABLE_ENCRYPTION=true TLS_CERT_PATH=/app/certs/cert.pem TLS_KEY_PATH=/app/certs/key.pem ./target/release/ruggine-server

Se vuoi, genero i file `Dockerfile.server` e `docker-compose.yml` di esempio e uno script PowerShell per creare certificati self-signed in ambiente Windows. Dimmi cosa preferisci e procedo.
