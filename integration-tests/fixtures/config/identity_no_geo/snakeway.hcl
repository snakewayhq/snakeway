server {
  version = 1
}

include {
  devices = "devices.d/*.hcl"
  ingress = "ingress.d/*.hcl"
}
