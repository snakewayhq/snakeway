server {
  version  = 1
  pid_file = "/run/snakeway/snakeway.pid"
}

include {
  devices   = "device.d/*.hcl"
  ingresses = "ingress.d/*.hcl"
}
