#!/bin/sh
set -e

CA_SERVER_URL=${CA_SERVER_URL:?CA_SERVER_URL must be set}
CERT_NAME=${CERT_NAME:=forward-proxy}
CA_PATH=${CA_PATH:=/certs/root_ca.crt}
CERT_PATH=${CERT_PATH:=/certs/client.crt}
KEY_PATH=${KEY_PATH:=/certs/client.key}

echo "Initializing directories..."
mkdir -p /certs "$(dirname "$CERT_PATH")" "$(dirname "$CA_PATH")"

#echo "Waiting for step-ca..."
#
#until curl -s ${CA_SERVER_URL}/health >/dev/null; do
#  sleep 1
#done

# ----------------------------
# Download root CA
# ----------------------------
echo "Downloading root CA..."

until curl -fsSL -k "${CA_SERVER_URL}/roots.pem" -o "${CA_PATH}"; do
  echo "Failed to download root CA. Retrying in 2s..."
  sleep 2
done

# ----------------------------
# Bootstrap step CLI
# ----------------------------
echo "Bootstrapping step CLI..."

step ca bootstrap \
  --ca-url "${CA_SERVER_URL}" \
  --fingerprint "$(step certificate fingerprint "${CA_PATH}")"

# ----------------------------
# Request certificate (ACME HTTP-01)
# ----------------------------
if [ ! -f "${CERT_PATH}" ] || [ ! -f "${KEY_PATH}" ]; then
  echo "Requesting certificate..."

  until step ca certificate \
    "${CERT_NAME}" \
    "${CERT_PATH}" \
    "${KEY_PATH}" \
    --ca-url "${CA_SERVER_URL}" \
    --root "${CA_PATH}" \
    --provisioner acme; do

    echo "Certificate request failed. Retrying in 5s..."
    sleep 5
  done

  echo "Certificate obtained"
else
  echo "Certificate already exists, skipping request"
fi

# ----------------------------
# Start renewal daemon (background)
# ----------------------------
echo "Starting automatic renewal daemon..."

exec step ca renew \
  ${CERT_PATH} \
  ${KEY_PATH} \
  --ca-url ${CA_SERVER_URL} \
  --root ${CA_PATH} \
  --daemon
