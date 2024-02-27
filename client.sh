#!/bin/bash

if [ -n "$CLIENT_DELAY" ]; then
  sleep "$CLIENT_DELAY"
fi

env ENABLE_PHYSICS="${ENABLE_PHYSICS:-true}" CONNECT_TO="localhost:3000" WINDOW_OFFSET="1000" cargo run
