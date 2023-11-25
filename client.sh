#!/bin/bash

if [ -n "$CLIENT_DELAY" ]; then
  sleep "$CLIENT_DELAY"
fi

env SERVE_ON="127.0.0.1:3001" CONNECT_TO="127.0.0.1:3000" WINDOW_OFFSET="800" cargo run
