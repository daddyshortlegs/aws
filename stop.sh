#!/bin/bash

# Check if services.pid file exists
if [ ! -f "services.pid" ]; then
    echo "No services.pid file found. Nothing to stop."
    exit 0
fi

echo "Stopping services..."

# Read each PID from the file and stop the process
while IFS= read -r pid; do
    if [ -n "$pid" ]; then
        echo "Stopping process with PID: $pid"
        
        # Check if the process is still running
        if kill -0 "$pid" 2>/dev/null; then
            # Try graceful termination first
            kill -TERM "$pid" 2>/dev/null
            
            # Wait a bit for graceful shutdown
            sleep 2
            
            # Check if process is still running
            if kill -0 "$pid" 2>/dev/null; then
                echo "Process $pid still running, force killing..."
                kill -KILL "$pid" 2>/dev/null
            else
                echo "Process $pid stopped gracefully"
            fi
        else
            echo "Process $pid is not running"
        fi
    fi
done < services.pid

echo "All services stopped."
