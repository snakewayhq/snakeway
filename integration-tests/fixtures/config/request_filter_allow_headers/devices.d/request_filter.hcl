request_filter_device {
  enable = true

  allow_headers = [
    "host",
    "x-custom-allowed",
    "accept",
    "accept-encoding",
    "user-agent",
    "content-length",
  ]
}
