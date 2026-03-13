#!/bin/bash
set -m

cd proxy && cargo run &
PID=$!
echo "Proxy running on PID: $PID"
echo $PID >> services.pid

cd backend && cargo run &
PID=$!
echo "Backend 1 running on PID: $PID"
echo $PID > services.pid

cd backend && APP_ENV=backend2 cargo run &
PID=$!
echo "Backend 2 running on PID: $PID"
echo $PID > services.pid

cd backend && APP_ENV=backend3 cargo run &
PID=$!
echo "Backend 3 running on PID: $PID"
echo $PID > services.pid

cd node-ssh && npm start &
PID=$!
echo "SSH server running on PID: $PID"
echo $PID >> services.pid

cd frontend && npm start &
PID=$!
echo "Frontend running on PID: $PID"
echo $PID >> services.pid
