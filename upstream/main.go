package main

import (
	"flag"
	"log"
	"net"
	"net/http"
	"os"
	"upstream/server"

	"golang.org/x/net/http2"
	googlegrpc "google.golang.org/grpc"
	"google.golang.org/grpc/credentials"
)

func getenv(key, fallback string) string {
	if v := os.Getenv(key); v != "" {
		return v
	}
	return fallback
}

func main() {
	certFile := flag.String(
		"tls-cert",
		getenv("TLS_CERT_FILE", "./data/certs/localhost-cert.pem"),
		"Path to TLS certificate PEM file",
	)

	keyFile := flag.String(
		"tls-key",
		getenv("TLS_KEY_FILE", "./data/certs/localhost-key.pem"),
		"Path to TLS private key PEM file",
	)

	flag.Parse()

	tlsCfg, err := server.NewTLSConfig(server.TLSOptions{
		CertFile: *certFile,
		KeyFile:  *keyFile,
	})
	if err != nil {
		log.Fatalf("TLS config error: %v", err)
	}

	httpHandler := server.NewHTTPHandler()

	// Start HTTP and WS Server (unencrypted).
	go func() {
		log.Println("Starting HTTP + WS on :3000")
		if err := http.ListenAndServe(":3000", httpHandler); err != nil {
			log.Fatalf("HTTP server failed: %v", err)
		}
	}()

	// Start HTTPS and WSS Server (TLS).
	go func() {
		httpsServer := &http.Server{
			Addr:      ":3443",
			Handler:   httpHandler,
			TLSConfig: tlsCfg,
		}

		// Enable HTTP/2 support.
		if err := http2.ConfigureServer(httpsServer, &http2.Server{}); err != nil {
			log.Fatalf("failed to configure http2: %v", err)
		}

		log.Println("Starting HTTPS + WSS on :3443")
		if err := httpsServer.ListenAndServeTLS("", ""); err != nil {
			log.Fatalf("HTTPS httpsServer failed: %v", err)
		}
	}()

	// Start gRPC Server (TLS, h2).
	go func() {
		lis, err := net.Listen("tcp", ":5051")
		if err != nil {
			log.Fatalf("failed to listen on :5051: %v", err)
		}

		creds := credentials.NewTLS(tlsCfg)
		grpcServer := googlegrpc.NewServer(googlegrpc.Creds(creds))

		// Register the manually defined gRPC service
		server.RegisterUserService(grpcServer)

		log.Println("Starting gRPC (TLS, h2) on :5051")
		if err := grpcServer.Serve(lis); err != nil {
			log.Fatalf("gRPC server failed: %v", err)
		}
	}()

	// Block forever to keep the goroutines running.
	select {}
}
