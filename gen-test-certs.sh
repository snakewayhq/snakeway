#!/usr/bin/env bash
set -euo pipefail

CERT_DIR="integration-tests/certs"
DAYS=3650

mkdir -p "${CERT_DIR}"

echo "Generating Snakeway test certificates in ${CERT_DIR}"

#------------------------------------------------------------------------------
# CA config
#------------------------------------------------------------------------------
cat > "${CERT_DIR}/ca.cnf" <<'EOF'
[req]
prompt = no
distinguished_name = dn
x509_extensions = v3_ca

[dn]
CN = Snakeway Dev Root CA (DO NOT TRUST IN PROD)

[v3_ca]
basicConstraints = critical, CA:TRUE
keyUsage = critical, keyCertSign, cRLSign
subjectKeyIdentifier = hash
EOF

# Generate CA key
openssl genrsa -out "${CERT_DIR}/ca.key" 4096

# Generate CA cert
openssl req -x509 -new -nodes \
  -key "${CERT_DIR}/ca.key" \
  -sha256 \
  -days "${DAYS}" \
  -out "${CERT_DIR}/ca.pem" \
  -config "${CERT_DIR}/ca.cnf"

#------------------------------------------------------------------------------
# Server (leaf) config
#------------------------------------------------------------------------------
cat > "${CERT_DIR}/server.cnf" <<'EOF'
[req]
prompt = no
distinguished_name = dn
req_extensions = v3_req

[dn]
CN = localhost

[v3_req]
basicConstraints = critical, CA:FALSE
keyUsage = critical, digitalSignature, keyEncipherment
extendedKeyUsage = serverAuth
subjectAltName = @alt_names
subjectKeyIdentifier = hash

[alt_names]
DNS.1 = localhost
IP.1  = 127.0.0.1
EOF

# Generate server key
openssl genrsa -out "${CERT_DIR}/server.key" 4096

# Create CSR
openssl req -new \
  -key "${CERT_DIR}/server.key" \
  -out "${CERT_DIR}/server.csr" \
  -config "${CERT_DIR}/server.cnf"

# Sign server cert with CA
openssl x509 -req \
  -in "${CERT_DIR}/server.csr" \
  -CA "${CERT_DIR}/ca.pem" \
  -CAkey "${CERT_DIR}/ca.key" \
  -CAcreateserial \
  -out "${CERT_DIR}/server.pem" \
  -days "${DAYS}" \
  -sha256 \
  -extfile "${CERT_DIR}/server.cnf" \
  -extensions v3_req

# Verify output exists
if [[ ! -f "${CERT_DIR}/server.pem" ]]; then
  echo "❌ ERROR: server.pem was not generated"
  exit 1
fi

# Cleanup
rm -f \
  "${CERT_DIR}/server.csr" \
  "${CERT_DIR}/server.cnf" \
  "${CERT_DIR}/ca.cnf" \
  "${CERT_DIR}/ca.srl"

echo "✔ Test certificates generated"
echo "  CA:     ${CERT_DIR}/ca.pem"
echo "  Server: ${CERT_DIR}/server.pem"
