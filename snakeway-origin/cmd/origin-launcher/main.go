package main

import (
	"context"
	"fmt"
	"log"
	"os"
	"os/exec"
	"os/signal"
	"syscall"
	"time"
)

func main() {
	const (
		basePort = 3000
		count    = 5
		binary   = "./origin-server"
	)

	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()

	var procs []*exec.Cmd

	for i := 0; i < count; i++ {
		port := basePort + i

		cmd := exec.CommandContext(
			ctx,
			binary,
			"-port", fmt.Sprint(port),
			"-tls-cert", "../integration-tests/certs/server.pem",
			"-tls-key", "../integration-tests/certs/server.key",
		)

		cmd.Stdout = os.Stdout
		cmd.Stderr = os.Stderr

		log.Printf("starting server on port %d", port)
		if err := cmd.Start(); err != nil {
			log.Fatalf("failed to start server: %v", err)
		}

		procs = append(procs, cmd)
		// avoid thundering herd
		time.Sleep(100 * time.Millisecond)
	}

	<-ctx.Done()
	log.Println("shutting down children...")

	for _, p := range procs {
		_ = p.Process.Signal(syscall.SIGTERM)
	}

	time.Sleep(500 * time.Millisecond)
}
