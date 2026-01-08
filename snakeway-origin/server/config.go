package server

import (
	"log"
	"os"
	"strconv"
)

type Config struct {
	Port       int
	CertFile   string
	KeyFile    string
	InstanceId int
}

func LoadConfig() Config {
	return Config{
		Port:       getenvInt("PORT", 3000),
		CertFile:   getenvStr("TLS_CERT_FILE", "./data/certs/server.pem"),
		KeyFile:    getenvStr("TLS_KEY_FILE", "./data/certs/server.key"),
		InstanceId: getenvInt("INSTANCE_ID", 0),
	}
}

func getenv(key string) (string, bool) {
	v, ok := os.LookupEnv(key)
	return v, ok
}

func getenvStr(key, fallback string) string {
	if v, ok := getenv(key); ok {
		return v
	}
	return fallback
}

func getenvInt(key string, fallback int) int {
	if v, ok := getenv(key); ok {
		if i, err := strconv.Atoi(v); err == nil {
			return i
		}
		log.Printf("invalid int for %s=%q, using fallback %d", key, v, fallback)
	}
	return fallback
}
