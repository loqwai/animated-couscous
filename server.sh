#!/bin/bash


# env SERVE_ON="127.0.0.1:3000" CONNECT_TO="127.0.0.1:3000" WINDOW_OFFSET="100" cargo run
env ENABLE_PHYSICS='true' SERVE_ON="0.0.0.0:3000" WINDOW_OFFSET="100" cargo run
