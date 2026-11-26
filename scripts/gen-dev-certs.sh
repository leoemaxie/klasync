#!/usr/bin/env sh
# Generates a throwaway CA and a Redis server certificate for the local stack.
# The backend refuses plaintext redis://, so local Redis must speak TLS too.
set -eu
dir="$(dirname "$0")/../deploy/certs"
mkdir -p "$dir"
if [ -f "$dir/ca.crt" ] && [ -f "$dir/redis.crt" ]; then
  echo "certs already exist in $dir"
  exit 0
fi
cd "$dir"
openssl genrsa -out ca.key 2048 2>/dev/null
openssl req -x509 -new -nodes -key ca.key -sha256 -days 825 -subj "/CN=klasync-dev-ca" -out ca.crt
openssl genrsa -out redis.key 2048 2>/dev/null
openssl req -new -key redis.key -subj "/CN=redis" -out redis.csr
printf "subjectAltName=DNS:redis,DNS:localhost,IP:127.0.0.1\nextendedKeyUsage=serverAuth\n" > redis.ext
openssl x509 -req -in redis.csr -CA ca.crt -CAkey ca.key -CAcreateserial -out redis.crt -days 825 -sha256 -extfile redis.ext 2>/dev/null
rm -f redis.csr redis.ext ca.srl
chmod 644 ca.crt redis.crt redis.key
echo "generated dev certs in $dir"
