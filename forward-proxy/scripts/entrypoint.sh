#!/bin/sh
set -e

echo "Starting certificate renewal..."
/scripts/renew_cert.sh &

echo "Waiting for initial certificate..."
until [ -f ${CERT_PATH} ]; do
  sleep 1
done

echo "Starting forward-proxy..."
forward-proxy
