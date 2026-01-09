structured_logging_device = {
  enable = true

  include_headers = false

  allowed_headers = []
  redacted_headers = []

  level = "info"

  include_identity = false

  identity_fields = [
    "country",
    "region",
    "asn",
    "device",
    "bot",
  ]
}
