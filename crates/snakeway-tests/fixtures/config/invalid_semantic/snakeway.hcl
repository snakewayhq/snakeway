
server {
    version = 999
}

include {
    ingresses = "ingress.d/*.hcl"
    devices   = "device.d/*.hcl"
}
