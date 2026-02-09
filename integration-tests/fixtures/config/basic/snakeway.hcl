server {
  version = 1
}

include {
  devices = "device.d/*.hcl"
  ingress = "ingress.d/*.hcl"
}
