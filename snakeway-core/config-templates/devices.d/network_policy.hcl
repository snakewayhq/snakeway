network_policy_device {
  cidr {
    allow = ["10.0.0.0/8"]
  }

  forwarding {
    # Allow forwarded requests at all?
    allow = true
    # If forwarded headers exist but identity says they are invalid
    on_invalid = "deny"  # deny | ignore
  }
}
