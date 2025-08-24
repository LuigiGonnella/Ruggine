# Implementazione della Crittografia in Ruggine

## Panoramica

Il sistema di chat Ruggine implementa la crittografia end-to-database per proteggere i messaggi memorizzati nel database SQLite. Tutti i messaggi sono crittografati prima di essere salvati e decrittografati solo quando vengono letti dagli utenti autorizzati.

## Architettura di Sicurezza

### 1. Algoritmo di Crittografia

- **Algoritmo**: AES-256-GCM (Advanced Encryption Standard con Galois/Counter Mode)
- **Dimensione chiave**: 256 bit (32 byte)
- **Dimensione nonce**: 96 bit (12 byte) - generato casualmente per ogni messaggio
- **Autenticazione**: GCM fornisce autenticazione integrata per prevenire manomissioni

### 2. Gestione delle Chiavi

#### Master Key
- Una chiave master di 256 bit viene generata o caricata all'avvio del server
- Memorizzata nella variabile di ambiente `ENCRYPTION_MASTER_KEY` (formato esadecimale)
- Se non presente, viene generata automaticamente e mostrata nei log per la persistenza

#### Chat-Specific Keys
- Ogni chat (privata o di gruppo) ha una chiave derivata univoca
- Generata usando SHA-256 da: `master_key + participant_ids_sorted`
- Garantisce che solo i partecipanti autorizzati possano decrittografare i messaggi

### 3. Struttura dei Dati Crittografati

I messaggi crittografati sono memorizzati come JSON nel database:

```json
{
  "ciphertext": "base64_encoded_encrypted_data",
  "nonce": "base64_encoded_nonce"
}
```

## Implementazione Tecnica

### 1. Modulo Crypto (`src/common/crypto.rs`)

```rust
pub struct CryptoManager;

impl CryptoManager {
    // Genera chiave master casuale
    pub fn generate_master_key() -> [u8; 32]
    
    // Crittografa un messaggio
    pub fn encrypt_message(plaintext: &str, key: &[u8; 32]) -> Result<(Vec<u8>, Vec<u8>), Unspecified>
    
    // Decrittografa un messaggio
    pub fn decrypt_message(ciphertext: &[u8], nonce: &[u8], key: &[u8; 32]) -> Result<String, Unspecified>
    
    // Genera chiave specifica per chat
    pub fn generate_chat_key(participants: &[String], master_key: &[u8; 32]) -> [u8; 32]
}
```

### 2. Processo di Crittografia

#### Invio Messaggio (Encryption Flow)

1. **Input**: Messaggio in plain text, lista partecipanti
2. **Key Generation**: Genera chiave chat-specifica da master key + partecipanti ordinati
3. **Nonce Generation**: Genera nonce casuale (12 byte)
4. **Encryption**: AES-256-GCM encrypts message con chiave e nonce
5. **Storage**: Salva JSON `{ciphertext, nonce}` in base64 nel database

#### Lettura Messaggio (Decryption Flow)

1. **Retrieval**: Legge JSON dal database
2. **Parsing**: Decodifica base64 per ottenere ciphertext e nonce
3. **Key Recreation**: Rigenera stessa chiave chat-specifica
4. **Decryption**: AES-256-GCM decrypts con chiave e nonce
5. **Validation**: GCM verifica autenticità automaticamente

### 3. Backward Compatibility

Il sistema gestisce messaggi legacy (non crittografati):

```rust
fn decrypt_message_from_storage(encrypted_data: &str, participants: &[String], config: &ServerConfig) -> Result<String, String> {
    // Tenta parsing JSON - se fallisce, è legacy plain text
    if let Ok(data) = serde_json::from_str::<serde_json::Value>(encrypted_data) {
        // Messaggio crittografato - decripta
        decrypt_encrypted_message(data, participants, config)
    } else {
        // Messaggio legacy - ritorna come plain text
        Ok(encrypted_data.to_string())
    }
}
```

## Sicurezza e Considerazioni

### 1. Punti di Forza

- **Crittografia forte**: AES-256-GCM è standard industriale
- **Autenticazione integrata**: GCM previene manomissioni
- **Isolamento delle chat**: Ogni chat ha chiave separata
- **Perfect Forward Secrecy**: Chiavi derivate, non riutilizzate

### 2. Gestione degli Errori

- Messaggi non decrittografabili mostrano `[DECRYPTION FAILED]`
- Log dettagliati per debugging senza esporre contenuti
- Graceful fallback per messaggi legacy

### 3. Configurazione

```env
# .env file
ENABLE_ENCRYPTION=true
ENCRYPTION_MASTER_KEY=a1b2c3d4e5f6789012345678901234567890abcdef1234567890abcdef123456
```

### 4. Limitazioni Attuali

- Master key deve essere consistente tra riavvii
- Messaggi crittografati con chiavi diverse non sono recuperabili
- Nessuna rotazione automatica delle chiavi

## Flusso dei Dati

```
[Client] --plaintext--> [Server] --encrypt--> [Database]
                                      |
                                   AES-256-GCM
                                      |
                                   JSON format
                                      
[Client] <--plaintext-- [Server] <--decrypt-- [Database]
                                      |
                                   Parse JSON
                                      |
                                   AES-256-GCM
```

## File Coinvolti

- `src/common/crypto.rs` - Implementazione crittografia
- `src/server/config.rs` - Gestione master key
- `src/server/messages.rs` - Encryption/decryption dei messaggi
- `.env` - Configurazione master key

## Logging e Debug

Il sistema include logging dettagliato per il debugging:

```
[CRYPTO] Encrypting message for participants: ["uuid1", "uuid2"]
[CRYPTO] Successfully encrypted message
[CRYPTO] Decrypting message for participants: ["uuid1", "uuid2"]
[CRYPTO] Successfully decrypted message
[CRYPTO] Decryption failed: Unspecified
```

## Considerazioni Future

1. **Key Rotation**: Implementare rotazione periodica delle chiavi
2. **Hardware Security**: Utilizzare HSM per master key storage
3. **End-to-End**: Estendere a crittografia end-to-end tra client
4. **Audit Trail**: Log crittografici per compliance
5. **Recovery**: Meccanismi di recovery per chiavi perse

## Conclusioni

L'implementazione fornisce una base solida per la sicurezza dei messaggi con crittografia moderna e pratiche standard. Il sistema è progettato per essere sicuro, performante e maintainabile, con chiara separazione tra logica di crittografia e business logic.
