server {
  version  = 1
  pid_file = "/tmp/snakeway.pid"
  threads  = 8
  ca_file  = "./integration-tests/certs/ca.pem"
}

include {
  devices = "devices.d/*.hcl"
  ingress = "ingress.d/*.hcl"
}
