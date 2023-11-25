#!/bin/bash

main() {
  cargo build \
  && env CLIENT_DELAY="0.5" \
    concurrently --kill-others ./server.sh ./client.sh
}
main "$@"

