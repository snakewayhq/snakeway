package main

import (
	"flag"
	"fmt"
	"log"
	"net"
	"net/http"
	"upstream/server"

	"golang.org/x/net/http2"
	googlegrpc "google.golang.org/grpc"
	"google.golang.org/grpc/credentials"
)

func main() {
	cfg := server.LoadConfig()

	flag.IntVar(&cfg.Port, "port", cfg.Port, "Base port")
	flag.StringVar(&cfg.CertFile, "tls-cert", cfg.CertFile, "TLS cert file")
	flag.StringVar(&cfg.KeyFile, "tls-key", cfg.KeyFile, "TLS key file")

	flag.Parse()

	tlsCfg, err := server.NewTLSConfig(server.TLSOptions{
		CertFile: cfg.CertFile,
		KeyFile:  cfg.KeyFile,
	})
	if err != nil {
		log.Fatalf("TLS config error: %v", err)
	}

	httpHandler := server.NewHTTPHandler()

	httpAddr := fmt.Sprintf(":%d", cfg.Port)
	httpsAddr := fmt.Sprintf(":%d", cfg.Port+443)
	grpcAddr := fmt.Sprintf(":%d", cfg.Port+2051)

	// Start HTTP and WS Server (unencrypted).
	go func() {
		log.Printf("Starting HTTP + WS on %s\n", httpAddr)
		if err := http.ListenAndServe(httpAddr, httpHandler); err != nil {
			log.Fatalf("HTTP server failed: %v", err)
		}
	}()

	// Start HTTPS and WSS Server (TLS).
	go func() {
		httpsServer := &http.Server{
			Addr:      httpsAddr,
			Handler:   httpHandler,
			TLSConfig: tlsCfg,
		}

		// Enable HTTP/2 support.
		if err := http2.ConfigureServer(httpsServer, &http2.Server{}); err != nil {
			log.Fatalf("failed to configure http2: %v", err)
		}

		log.Printf("Starting HTTPS + WSS on %s\n", httpsAddr)
		if err := httpsServer.ListenAndServeTLS("", ""); err != nil {
			log.Fatalf("HTTPS httpsServer failed: %v", err)
		}
	}()

	// Start gRPC Server (TLS, h2).
	go func() {
		lis, err := net.Listen("tcp", grpcAddr)
		if err != nil {
			log.Fatalf("failed to listen on %s: %v", grpcAddr, err)
		}

		creds := credentials.NewTLS(tlsCfg)
		grpcServer := googlegrpc.NewServer(googlegrpc.Creds(creds))

		// Register the manually defined gRPC service
		server.RegisterUserService(grpcServer)

		log.Printf("Starting gRPC (TLS, h2) on %s\n", grpcAddr)
		if err := grpcServer.Serve(lis); err != nil {
			log.Fatalf("gRPC server failed: %v", err)
		}
	}()

	// Block forever to keep the goroutines running.
	select {}
}
