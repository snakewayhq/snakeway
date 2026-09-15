#!/usr/bin/env bash
set -euo pipefail

# Key format fixtures for the tests in crates/snakeway-conf/src/tls.rs.
# rcgen only writes PKCS#8 keys, so run this once with OpenSSL 3 to produce PKCS#1,
# SEC1, and encrypted keys, then commit the output. The tests do not check dates.

FIXTURE_DIR="crates/snakeway-conf/fixtures/tls"
DAYS=36500

mkdir -p "${FIXTURE_DIR}"

echo "Generating TLS key format fixtures in ${FIXTURE_DIR}"

openssl genrsa -traditional -out "${FIXTURE_DIR}/rsa-pkcs1.key" 2048
openssl req -x509 -new \
  -key "${FIXTURE_DIR}/rsa-pkcs1.key" \
  -sha256 \
  -days "${DAYS}" \
  -subj "/CN=localhost" \
  -addext "subjectAltName=DNS:localhost" \
  -out "${FIXTURE_DIR}/rsa.pem"

openssl ecparam -name prime256v1 -genkey -noout -out "${FIXTURE_DIR}/ec-sec1.key"
openssl req -x509 -new \
  -key "${FIXTURE_DIR}/ec-sec1.key" \
  -sha256 \
  -days "${DAYS}" \
  -subj "/CN=localhost" \
  -addext "subjectAltName=DNS:localhost" \
  -out "${FIXTURE_DIR}/ec.pem"

openssl req -x509 -new \
  -key "${FIXTURE_DIR}/ec-sec1.key" \
  -sha256 \
  -days "${DAYS}" \
  -subj "/CN=localhost" \
  -addext "subjectAltName=DNS:localhost" \
  -addext "1.3.6.1.4.1.55555.1=critical,ASN1:UTF8String:snakeway" \
  -out "${FIXTURE_DIR}/ec-unknown-critical-extension.pem"

openssl ecparam -name secp521r1 -genkey -noout -out "${FIXTURE_DIR}/ec-p521-sec1.key"

openssl pkcs8 -topk8 -v2 aes-256-cbc \
  -in "${FIXTURE_DIR}/ec-sec1.key" \
  -passout pass:snakeway-test \
  -out "${FIXTURE_DIR}/ec-encrypted-pkcs8.key"

echo "TLS key format fixtures generated"
