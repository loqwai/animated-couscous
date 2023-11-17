#!/bin/bash


env SERVE_ON="127.0.0.1:3193" CONNECT_TO="127.0.0.1:3192" WINDOW_OFFSET="900" cargo run
