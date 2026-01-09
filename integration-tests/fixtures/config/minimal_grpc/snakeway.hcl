server {
  version = 1
  threads = 1
  ca_file = "./certs/ca.pem"
}

include {
  devices = "devices.d/*.hcl"
  ingress = "ingress.d/*.hcl"
}
