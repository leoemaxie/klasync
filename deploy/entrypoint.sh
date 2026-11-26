#!/bin/sh
# Builds a CA bundle = system roots + the dev CA (+ any extra bundle mounted at
# /extra-ca.crt, e.g. a corporate proxy), then starts the backend.
set -eu
mkdir -p /tmp/certs
cat /etc/ssl/certs/ca-certificates.crt /certs-src/ca.crt > /tmp/certs/ca-bundle.crt
if [ -f /extra-ca.crt ]; then cat /extra-ca.crt >> /tmp/certs/ca-bundle.crt; fi
export SSL_CERT_FILE=/tmp/certs/ca-bundle.crt
# The HTTP client (Gemini/OpenRouter) trusts the same bundle.
export HTTP_EXTRA_CA_FILE="${HTTP_EXTRA_CA_FILE:-/tmp/certs/ca-bundle.crt}"
exec /app/klasync-backend
