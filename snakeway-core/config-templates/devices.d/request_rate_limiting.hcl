request_rate_limiting_device {
  enable                  = true
  window_seconds = 3  # 3 second window
  max_requests_per_second = 100  # Maximum number of requests in a 3 second window
}
