#!/bin/bash

main() {
  cargo run --bin generate_protobuf_client
}
main "$@"