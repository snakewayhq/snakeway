package main

import (
	"context"
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

	flag.IntVar(&cfg.Port, "port", cfg.Port, "Base port")
	flag.StringVar(&cfg.CertFile, "tls-cert", cfg.CertFile, "TLS cert file")
	flag.StringVar(&cfg.KeyFile, "tls-key", cfg.KeyFile, "TLS key file")
	flag.Parse()

	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()

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

	httpSrv := &http.Server{
		Addr:    httpAddr,
		Handler: httpHandler,
	}

	httpsSrv := &http.Server{
		Addr:      httpsAddr,
		Handler:   httpHandler,
		TLSConfig: tlsCfg,
	}

	// HTTP over UDS
	httpSock := "/tmp/snakeway-http.sock"
	_ = os.Remove(httpSock)
	httpUdsLis, err := net.Listen("unix", httpSock)
	if err != nil {
		log.Fatalf("failed to listen on HTTP UDS %s: %v", httpSock, err)
	}
	log.Printf("Listening HTTP + WS on UDS %s\n", httpSock)

	_ = os.Chmod(httpSock, 0660)

	// HTTP over UDS
	go func() {
		if err := httpSrv.Serve(httpUdsLis); err != nil && err != http.ErrServerClosed {
			log.Fatalf("HTTP UDS server failed: %v", err)
		}
	}()

	// HTTP
	go func() {
		log.Printf("Starting HTTP + WS on %s\n", httpAddr)
		if err := httpSrv.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			log.Fatalf("HTTP server failed: %v", err)
		}
	}()

	// HTTPS
	go func() {
		if err := http2.ConfigureServer(httpsSrv, &http2.Server{}); err != nil {
			log.Fatalf("failed to configure http2: %v", err)
		}

		log.Printf("Starting HTTPS + WSS on %s\n", httpsAddr)
		if err := httpsSrv.ListenAndServeTLS("", ""); err != nil && err != http.ErrServerClosed {
			log.Fatalf("HTTPS server failed: %v", err)
		}
	}()

	// gRPC
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

	// Wait for a signal to stop...
	<-ctx.Done()
	log.Println("shutting down origin-server")

	shutdownCtx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	_ = httpSrv.Shutdown(shutdownCtx)
	_ = httpsSrv.Shutdown(shutdownCtx)
	grpcServer.GracefulStop()
}
