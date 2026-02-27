#!/bin/bash
set -m

cd backend && cargo run &
PID=$!
echo "Backend running on PID: $PID"
echo $PID > services.pid

cd proxy && cargo run &
PID=$!
echo "Proxy running on PID: $PID"
echo $PID >> services.pid

cd node-ssh && npm start &
PID=$!
echo "SSH server running on PID: $PID"
echo $PID >> services.pid

cd frontend && npm start &
PID=$!
echo "Frontend running on PID: $PID"
echo $PID >> services.pid
