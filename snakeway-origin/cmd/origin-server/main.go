package main

import (
	"context"
	"crypto/tls"
	"flag"
	"fmt"
	"log"
	"net"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"upstream/server"

	"golang.org/x/net/http2"
	googlegrpc "google.golang.org/grpc"
	"google.golang.org/grpc/credentials"
)

func main() {
	cfg := server.LoadConfig()

	flag.IntVar(&cfg.InstanceId, "instance-id", 0, "Instance ID")
	flag.IntVar(&cfg.Port, "port", cfg.Port, "Base port")
	flag.StringVar(&cfg.CertFile, "tls-cert", cfg.CertFile, "TLS cert file")
	flag.StringVar(&cfg.KeyFile, "tls-key", cfg.KeyFile, "TLS key file")
	flag.Parse()

	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()

	// -------------------------------------------------------------------------
	// TLS config
	// -------------------------------------------------------------------------
	tlsCfg, err := server.NewTLSConfig(server.TLSOptions{
		CertFile: cfg.CertFile,
		KeyFile:  cfg.KeyFile,
	})
	if err != nil {
		log.Fatalf("TLS config error: %v", err)
	}

	// -------------------------------------------------------------------------
	// HTTP handler
	// -------------------------------------------------------------------------
	httpHandler := server.NewHTTPHandler()

	httpSrv := &http.Server{
		Handler: httpHandler,
	}

	httpsSrv := &http.Server{
		Handler:   httpHandler,
		TLSConfig: tlsCfg,
	}

	// Enable HTTP/2 on all TLS servers
	if err := http2.ConfigureServer(httpsSrv, &http2.Server{}); err != nil {
		log.Fatalf("failed to configure http2: %v", err)
	}

	// -------------------------------------------------------------------------
	// HTTP over TCP
	// -------------------------------------------------------------------------
	httpAddr := fmt.Sprintf(":%d", cfg.Port)
	go func() {
		log.Printf("Starting HTTP + WS on %s\n", httpAddr)
		if err := httpSrv.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			log.Fatalf("HTTP server failed: %v", err)
		}
	}()

	// -------------------------------------------------------------------------
	// HTTPS over TCP
	// -------------------------------------------------------------------------
	httpsAddr := fmt.Sprintf(":%d", cfg.Port+443)
	go func() {
		log.Printf("Starting HTTPS + WSS on %s\n", httpsAddr)
		if err := httpsSrv.ListenAndServeTLS("", ""); err != nil && err != http.ErrServerClosed {
			log.Fatalf("HTTPS server failed: %v", err)
		}
	}()

	// -------------------------------------------------------------------------
	// HTTP over UDS (plaintext)
	// -------------------------------------------------------------------------
	httpSock := fmt.Sprintf("/tmp/snakeway-http-%d.sock", cfg.InstanceId)
	_ = os.Remove(httpSock)

	httpUdsLis, err := net.Listen("unix", httpSock)
	if err != nil {
		log.Fatalf("failed to listen on HTTP UDS %s: %v", httpSock, err)
	}
	_ = os.Chmod(httpSock, 0660)

	log.Printf("Listening HTTP + WS on UDS %s\n", httpSock)
	go func() {
		if err := httpSrv.Serve(httpUdsLis); err != nil && err != http.ErrServerClosed {
			log.Fatalf("HTTP UDS server failed: %v", err)
		}
	}()

	// -------------------------------------------------------------------------
	// HTTPS over UDS (TLS + HTTP/2)
	// -------------------------------------------------------------------------
	httpsSock := fmt.Sprintf("/tmp/snakeway-https-%d.sock", cfg.InstanceId)
	_ = os.Remove(httpsSock)

	httpsUdsLis, err := net.Listen("unix", httpsSock)
	if err != nil {
		log.Fatalf("failed to listen on HTTPS UDS %s: %v", httpsSock, err)
	}
	_ = os.Chmod(httpsSock, 0660)

	tlsUdsLis := tls.NewListener(httpsUdsLis, tlsCfg)

	log.Printf("Listening HTTPS + WSS on UDS %s\n", httpsSock)
	go func() {
		if err := httpsSrv.Serve(tlsUdsLis); err != nil && err != http.ErrServerClosed {
			log.Fatalf("HTTPS UDS server failed: %v", err)
		}
	}()

	// -------------------------------------------------------------------------
	// gRPC over TCP (TLS, h2)
	// -------------------------------------------------------------------------
	grpcAddr := fmt.Sprintf(":%d", cfg.Port+2051)
	grpcLis, err := net.Listen("tcp", grpcAddr)
	if err != nil {
		log.Fatalf("failed to listen on %s: %v", grpcAddr, err)
	}

	grpcServer := googlegrpc.NewServer(
		googlegrpc.Creds(credentials.NewTLS(tlsCfg)),
	)
	server.RegisterUserService(grpcServer)

	go func() {
		log.Printf("Starting gRPC (TLS, h2) on %s\n", grpcAddr)
		if err := grpcServer.Serve(grpcLis); err != nil {
			log.Fatalf("gRPC server failed: %v", err)
		}
	}()

	// -------------------------------------------------------------------------
	// Shutdown
	// -------------------------------------------------------------------------
	<-ctx.Done()
	log.Println("shutting down origin-server")

	shutdownCtx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	_ = httpSrv.Shutdown(shutdownCtx)
	_ = httpsSrv.Shutdown(shutdownCtx)
	grpcServer.GracefulStop()
}
