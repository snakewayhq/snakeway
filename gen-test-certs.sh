#!/usr/bin/env bash
set -euo pipefail

CERT_DIR="integration-tests/certs"
DAYS=3650

mkdir -p "${CERT_DIR}"

echo "→ Generating Snakeway test certificates in ${CERT_DIR}"

#------------------------------------------------------------------------------
# Generate CA key
#------------------------------------------------------------------------------
openssl genrsa -out "${CERT_DIR}/ca.key" 4096

#------------------------------------------------------------------------------
# Generate CA cert
#------------------------------------------------------------------------------
openssl req -x509 -new -nodes \
  -key "${CERT_DIR}/ca.key" \
  -sha256 \
  -days "${DAYS}" \
  -out "${CERT_DIR}/ca.pem" \
  -subj "/CN=Snakeway Test CA"

#------------------------------------------------------------------------------
# Generate server key
#------------------------------------------------------------------------------
openssl genrsa -out "${CERT_DIR}/server.key" 4096

#------------------------------------------------------------------------------
# OpenSSL config for SANs
#------------------------------------------------------------------------------
cat > "${CERT_DIR}/server.cnf" <<'EOF'
[req]
default_bits       = 4096
prompt             = no
default_md         = sha256
distinguished_name = dn
req_extensions     = req_ext

[dn]
CN = localhost

[req_ext]
subjectAltName = @alt_names

[alt_names]
DNS.1 = localhost
IP.1  = 127.0.0.1
EOF

#------------------------------------------------------------------------------
# Create CSR
#------------------------------------------------------------------------------
openssl req -new \
  -key "${CERT_DIR}/server.key" \
  -out "${CERT_DIR}/server.csr" \
  -config "${CERT_DIR}/server.cnf"

#------------------------------------------------------------------------------
# Sign server cert with CA
#------------------------------------------------------------------------------
openssl x509 -req \
  -in "${CERT_DIR}/server.csr" \
  -CA "${CERT_DIR}/ca.pem" \
  -CAkey "${CERT_DIR}/ca.key" \
  -CAcreateserial \
  -out "${CERT_DIR}/server.pem" \
  -days "${DAYS}" \
  -sha256 \
  -extfile "${CERT_DIR}/server.cnf" \
  -extensions req_ext

#------------------------------------------------------------------------------
# Cleanup
#------------------------------------------------------------------------------
rm -f \
  "${CERT_DIR}/server.csr" \
  "${CERT_DIR}/server.cnf" \
  "${CERT_DIR}/ca.srl"

echo "✓ Test certificates generated"
echo "  CA:     ${CERT_DIR}/ca.pem"
echo "  Server: ${CERT_DIR}/server.pem"
