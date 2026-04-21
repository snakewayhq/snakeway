structured_logging_device = {
  enable = false

  # Attach request/response headers to log lines.
  include_headers = false

  # When include_headers = true, only these headers are logged.
  allowed_headers = []

  # When include_headers = true, these headers are replaced with "[REDACTED]".
  redacted_headers = []

  # Log level for request events: "trace" | "debug" | "info" | "warn" | "error"
  level = "info"

  # Include identity enrichment fields (GeoIP, user-agent) in log lines.
  include_identity = false

  identity_fields = [
    "country",
    "region",
    "asn",
    "device",
    "bot",
  ]
}
