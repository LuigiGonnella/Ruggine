# TLS Configuration for Production

## Overview
Ruggine supports TLS/SSL encryption for secure communication. This guide explains how to configure TLS for production deployment.

## Prerequisites
1. A valid SSL certificate for your domain
2. Private key file corresponding to the certificate
3. Certificate files in PEM format

## Configuration Steps

### 1. Obtain SSL Certificates

#### Option A: Let's Encrypt (Recommended for most cases)
```bash
# Install certbot
sudo apt-get install certbot

# Get certificate for your domain
sudo certbot certonly --standalone -d yourdomain.com

# Certificates will be saved to:
# /etc/letsencrypt/live/yourdomain.com/cert.pem
# /etc/letsencrypt/live/yourdomain.com/privkey.pem
```

#### Option B: Commercial Certificate
- Purchase from a trusted CA (e.g., DigiCert, GlobalSign, etc.)
- Follow provider's instructions to generate and download certificates

### 2. Configure Environment Variables

Edit your `.env` file:

```bash
# Enable TLS
ENABLE_ENCRYPTION=true

# Set certificate paths
TLS_CERT_PATH=/etc/letsencrypt/live/yourdomain.com/cert.pem
TLS_KEY_PATH=/etc/letsencrypt/live/yourdomain.com/privkey.pem

# Production server settings
SERVER_HOST=0.0.0.0
SERVER_PORT=443  # Standard HTTPS port
```

### 3. File Permissions

Ensure proper permissions for certificate files:

```bash
# Let's Encrypt certificates
sudo chmod 644 /etc/letsencrypt/live/yourdomain.com/cert.pem
sudo chmod 600 /etc/letsencrypt/live/yourdomain.com/privkey.pem

# Make sure your application can read the files
sudo chown ruggine:ruggine /etc/letsencrypt/live/yourdomain.com/*
```

### 4. Firewall Configuration

Open the appropriate ports:

```bash
# Allow HTTPS traffic
sudo ufw allow 443/tcp

# If using custom port, replace 443 with your port
sudo ufw allow 5000/tcp
```

### 5. Start the Server

```bash
# Run the server
cargo run --bin ruggine-server --release
```

## Testing TLS Configuration

### Test with OpenSSL
```bash
# Test TLS connection
openssl s_client -connect yourdomain.com:443 -servername yourdomain.com

# Should show certificate details and successful connection
```

### Test with curl
```bash
# Test HTTPS endpoint
curl -v https://yourdomain.com:443
```

## Troubleshooting

### Common Issues

1. **"No private keys found"**
   - Check file paths in environment variables
   - Verify file permissions
   - Ensure key file is in PEM format

2. **"Certificate chain error"**
   - Make sure you're using the full certificate chain
   - For Let's Encrypt, use `fullchain.pem` instead of `cert.pem`

3. **"Permission denied"**
   - Check file ownership and permissions
   - Run server with appropriate user privileges

### Certificate Renewal (Let's Encrypt)

Set up automatic renewal:

```bash
# Add to crontab
0 12 * * * /usr/bin/certbot renew --quiet && systemctl restart ruggine-server
```

## Production Deployment Checklist

- [ ] Valid SSL certificate obtained
- [ ] Certificate files have correct permissions
- [ ] Environment variables configured
- [ ] Firewall rules updated
- [ ] DNS points to your server
- [ ] Automatic certificate renewal configured
- [ ] TLS connection tested
- [ ] Server logs show "TLS enabled and configured successfully"

## Security Considerations

1. **Keep certificates secure**: Restrict access to private key files
2. **Regular updates**: Keep rustls and related dependencies updated
3. **Monitor expiration**: Set up alerts for certificate expiration
4. **Strong ciphers**: The default rustls configuration uses secure cipher suites
5. **HSTS**: Consider implementing HTTP Strict Transport Security headers

## Support

- Check server logs for TLS-related errors
- Verify certificate validity with online tools
- Ensure all dependencies are up to date
