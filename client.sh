#!/bin/bash


env SERVE_ON="127.0.0.1:3192" CONNECT_TO="127.0.0.1:3191" WINDOW_OFFSET="500" cargo run
