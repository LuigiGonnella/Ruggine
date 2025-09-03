<p align="center">
	<img src="./img/ruggineImage.png" alt="Ruggine logo" width="420" />
</p>

# Ruggine — Chat client/server

Ruggine è una piattaforma di messaggistica moderna, end-to-end, progettata per essere sicura, modulare e operabile in ambienti di produzione. Il codice server è scritto in Rust usando Tokio e SQLx; il client desktop utilizza Iced per l'interfaccia.

Questa guida fornisce istruzioni chiare e operative su come configurare, costruire, distribuire e gestire Ruggine in produzione.

## Sommario
- Panoramica
- Requisiti di produzione
- Configurazione e gestione dei segreti
- Build, containerizzazione e deploy
- Operazioni, logging e monitoraggio
- Sicurezza e gestione delle chiavi crittografiche
- Backup, migrazioni e disaster recovery
- Scaling e architettura di produzione
- Troubleshooting e FAQ
- Contribuire

## Panoramica
Ruggine gestisce chat private e di gruppo con messaggistica in tempo reale. Le conversazioni sono salvate nel database in forma cifrata (AES-256-GCM) e il server mantiene un modello di sessioni e presenza per i client connessi. 

### Nuove Funzionalità v2.0
- **WebSocket + Redis**: Messaggistica in tempo reale che sostituisce il polling del database
- **Scalabilità migliorata**: Supporto per multiple istanze server via Redis pub/sub
- **Latenza ridotta**: Messaggi istantanei invece di attesa polling
- **Efficienza di rete**: Solo messaggi necessari invece di query periodiche

Il progetto è pensato per essere facilmente integrato in pipeline CI/CD e in infrastrutture containerizzate.

## Requisiti di produzione
- Toolchain: utilizzare Rust stable (compilare in CI). Bloccare le dipendenze con `Cargo.lock`.
- Database: PostgreSQL 14+ (consigliato); SQLite è solo per sviluppo.
- Redis: Redis 6+ per WebSocket pub/sub e caching (obbligatorio per messaging real-time).
- TLS: certificati validi per ingress/endpoint. È raccomandato l'uso di rustls o di un reverse-proxy (nginx/traefik).
- Secret management: Vault, AWS Secrets Manager, Azure Key Vault o equivalenti per `ENCRYPTION_MASTER_KEY` e credenziali DB.

## Configurazione e gestione dei segreti
I parametri principali sono gestiti tramite variabili d'ambiente (o secret mounts). Esempio minimo:

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

Linee guida operative:
- Non salvare chiavi o credenziali nel repository né in image non protette.
- Memorizzare `ENCRYPTION_MASTER_KEY` nel secret manager della piattaforma; caricarla al boot dell'applicazione.
- Se si ruota la `ENCRYPTION_MASTER_KEY`, assicurarsi di avere procedura per la migrazione o per mantenere chiavi legacy per poter decrittare messaggi storici (vedi `doc/ENCRYPTION.md`).

## Build, containerizzazione e deploy
Si raccomanda di costruire i binari in un job CI dedicato e di distribuire immagini Docker immutabili.

- Esempio di build in CI:

```powershell
cargo build --release --locked
```

- Dockerfile di esempio (multi-stage):

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

- Deployment consigliato:
	- Per PoC: `docker-compose` con Postgres e reverse-proxy TLS.
	- Per produzione: Kubernetes con Deployment, Service, Ingress, e Secret per `ENCRYPTION_MASTER_KEY`.

## Operazioni, logging e monitoraggio
- Logging: utilizzare formato strutturato (JSON) e centralizzare. `LOG_LEVEL` gestisce il livello di verbosità.
- Metriche: esporre metriche compatibili Prometheus (latency, message_count, decryption_errors, active_connections).
- Health checks: implementare `/healthz` e `/readyz` per le probe di orchestratori.
- Backup: eseguire backup regolari del DB e testare il ripristino. Automatizzare snapshot e retention policy.

## Sicurezza e gestione delle chiavi crittografiche
- Crittografia: AES-256-GCM per i payload dei messaggi. I messaggi sono memorizzati come JSON con `nonce`, `ciphertext` e metadati.
- Protezione chiave: mantenere `ENCRYPTION_MASTER_KEY` in un vault. L'accesso deve essere ristretto e auditabile.
- Rotazione chiave: progettare una strategia (rolling re-encrypt, mantenimento chiavi legacy). Documentazione tecnica in `doc/ENCRYPTION.md`.

## Backup, migrazioni e disaster recovery
- Migrazioni: tenere le migration files versionate e applicarle in CI con controllo dello schema.
- Recovery plan: scriptati i passi per restore DB, import della `ENCRYPTION_MASTER_KEY` e verifica integrità delle entità cifrate.

## Scaling e architettura di produzione
- Server: stateless, scalabile orizzontalmente dietro LB.
- Database: PostgreSQL con replica e backup; valutare partitioning per dataset massicci.
- Consigli: caching layer (Redis) per metadati ad accesso frequente e rate-limiting su ingress.

## Troubleshooting e FAQ
- Q: "I messaggi non si decriptano dopo un riavvio" — A: verificare `ENCRYPTION_MASTER_KEY` e cercare nel log entry con tag `[DECRYPTION FAILED]`.
- Q: "Registrazione fallita con username già in uso" — A: server restituisce errore user-friendly `ERR: Username già in uso`.
- Q: "Messaggi duplicati o perdita di presenza" — A: controllare il processo di polling/ack nel `ChatService` e le probe di rete.

## CI / Test suggeriti
- Unit tests: derivazione chiavi, encrypt/decrypt, e helper crittografici.
- Integration tests: job CI che esegue Postgres temporaneo, applica migrations e simula flussi di chat.

## Contribuire
- Branching: feature/*, fix/*, release/*.
- PR: includere descrizione, test e passi per verifica.

## Licenza e contatti
- Inserire il file `LICENSE` nella root per chiarire termini di utilizzo (MIT/Apache-2.0 consigliate).
- Mantainers: Luigi Gonnella & Dorotea Monaco — apri issue o PR nel repository per domande tecniche.

---