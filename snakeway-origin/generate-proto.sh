#!/usr/bin/env bash

mkdir -p ./userspb/
~/go/bin/protoc \
  --go_out=paths=source_relative:./userspb \
  --go-grpc_out=paths=source_relative:./userspb \
  users.proto