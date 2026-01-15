bind_admin = {
  interface = "127.0.0.1"
  port      = 8440
  tls = {
    cert = "./integration-tests/certs/server.pem"
    key  = "./integration-tests/certs/server.key"
  }
}
