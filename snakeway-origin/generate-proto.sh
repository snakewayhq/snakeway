#!/usr/bin/env bash

mkdir -p ./userspb/
protoc \
  --go_out=paths=source_relative:./userspb \
  --go-grpc_out=paths=source_relative:./userspb \
  users.proto