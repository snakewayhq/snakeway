request_filter_device {
  enable = true

  #----------------------------------------------------------------------------
  # Method policy
  #----------------------------------------------------------------------------
  allow_methods = ["GET", "POST"]
  # deny_methods = ["TRACE", "CONNECT"]

  #----------------------------------------------------------------------------
  # Header policy
  #----------------------------------------------------------------------------
  deny_headers = [
    "x-forwarded-host",
    "x-original-url",
  ]

  required_headers = [
    "host",
    "user-agent",
  ]

  # Header allowlist is intentionally omitted
  # (deny and required is the safe default)
  # allow_deny_headers = []

  #----------------------------------------------------------------------------
  # Size limits
  #----------------------------------------------------------------------------
  max_header_bytes = 16384          # 16 KB
  max_body_bytes = 1048576          # 1 MB
  max_suspicious_body_bytes = 8192  # 8 KB

  #----------------------------------------------------------------------------
  # Deny behavior
  #----------------------------------------------------------------------------
  deny_status = 403
}
