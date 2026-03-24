#!/bin/sh
set -e

CA_SERVER_URL=${CA_SERVER_URL:?CA_SERVER_URL must be set}
CERT_NAME=${CERT_NAME:=proxy}
CA_PATH=${CA_PATH:=/certs/root_ca.crt}
CERT_PATH=${CERT_PATH:=/certs/mtls.crt}
KEY_PATH=${KEY_PATH:=/certs/mtls.key}
SANS=${SANS:=""}
PROVISIONER=${PROVISIONER:=acme}
RENEW_INTERVAL=${RENEW_INTERVAL:=60}   # seconds between checks
RETRY_DELAY=${RETRY_DELAY:=10}         # retry delay on failure

TMP_CERT="${CERT_PATH}.new"
TMP_KEY="${KEY_PATH}.new"


log() {
  echo "[$(date -u '+%Y-%m-%d %H:%M:%S')] TLS: $*"
}

log "Initializing directories..."
mkdir -p /certs "$(dirname "$CERT_PATH")" "$(dirname "$CA_PATH")"

# ----------------------------
# Download root CA
# ----------------------------
log "Downloading root CA..."

until curl -fsSL -k "${CA_SERVER_URL}/roots.pem" -o "${CA_PATH}"; do
  log "Failed to download root CA. Retrying in 2s..."
  sleep 2
done

issue_cert() {
  log "Issuing new certificate..."

  # Build argument list safely
  set -- \
    "$CERT_NAME" \
    "$TMP_CERT" \
    "$TMP_KEY" \
    --ca-url "$CA_SERVER_URL" \
    --root "$CA_PATH" \
    --provisioner "$PROVISIONER"

  # Add SANs only if provided
  if [ -n "$SANS" ]; then
    for san in $SANS; do
      set -- "$@" --san "$san"
    done
  fi

  until step ca certificate "$@"; do
    log "Failed to issue certificate, retry in ${RETRY_DELAY}..."
    sleep "$RETRY_DELAY"
  done

  mv "$TMP_CERT" "$CERT_PATH"
  mv "$TMP_KEY" "$KEY_PATH"

  log "New certificate installed"
}

renew_cert() {
  log "Attempting certificate renewal..."

  if step ca renew \
    "$CERT_PATH" \
    "$KEY_PATH" \
    --ca-url "$CA_SERVER_URL" \
    --root "$CA_PATH" \
    --force; then

    log "Certificate renewed successfully"
    return 0
  else
    log "Renewal failed"
    return 1
  fi
}

cert_exists() {
  [ -f "$CERT_PATH" ] && [ -f "$KEY_PATH" ]
}

cert_expired() {
  if ! cert_exists; then
    log "No certificate found"
    return 0
  fi

  EXP_TS=$(openssl x509 -enddate -noout -in "$CERT_PATH" \
    | cut -d= -f2 \
    | xargs -I{} date -u -d "{}" +%s 2>/dev/null)

  if [ -z "$EXP_TS" ]; then
    log "Failed to read expiration → treat as expired"
    return 0
  fi

  NOW_TS=$(date -u +%s)
  REMAIN=$((EXP_TS - NOW_TS))
  HOUR=$((REMAIN / 3600))
  MIN=$((REMAIN / 60))
  SEC=$((REMAIN % 60))
  log "REMAIN: ${HOUR}h${MIN}m${SEC}s"

  [ "$EXP_TS" -le $((NOW_TS + RENEW_INTERVAL + 30)) ]
}

# ---- MAIN LOOP ----

log "Starting certificate manager..."

while true; do
  # Bootstrap: ensure cert exists
  if ! cert_exists; then
    log "No certificate found → issuing initial cert"
    issue_cert
  fi


  if cert_expired; then
    log "Certificate expired or about to expire"

    if ! renew_cert; then
      log "Renew failed → falling back to new certificate"
      issue_cert
    fi
  fi

  sleep "$RENEW_INTERVAL"
done
