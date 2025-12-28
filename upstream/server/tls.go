package server

import "crypto/tls"


type TLSOptions struct {
	CertFile string
	KeyFile  string
}


// NewTLSConfig loads certificates and returns a secure TLS configuration
// for HTTP/2, HTTP/1.1, and gRPC servers.
func NewTLSConfig(opts TLSOptions) (*tls.Config, error) {
	cert, err := tls.LoadX509KeyPair(opts.CertFile, opts.KeyFile)
	if err != nil {
		return nil, err
	}

	return &tls.Config{
		Certificates: []tls.Certificate{cert},
		NextProtos:   []string{"h2", "http/1.1"},
		MinVersion:  tls.VersionTLS12,
	}, nil
}