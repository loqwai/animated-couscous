#!/bin/bash

if [ -n "$CLIENT_DELAY" ]; then
  sleep "$CLIENT_DELAY"
fi

env ENABLE_PHYSICS='false' SERVE_ON="127.0.0.1:3001" CONNECT_TO="127.0.0.1:3000" WINDOW_OFFSET="1000" cargo run
# env CONNECT_TO="127.0.0.1:3000" WINDOW_OFFSET="1000" cargo run
