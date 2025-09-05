# Andy's Web Services (AWS)

A comprehensive virtual machine management system built with Rust, React, and Node.js. This system provides a web-based interface for launching, managing, and connecting to virtual machines with integrated SSH terminal access.

## System Architecture

The VM Orchestrator consists of several interconnected components:

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│   Frontend  │◄──►│    Proxy    │◄──►│   Backend   │
│  (Port 3000)│    │ (Port 8080) │    │(Port 8081)  │
└─────────────┘    └─────────────┘    └─────────────┘
       │                   │                   │
       │                   │                   │
       ▼                   ▼                   ▼
┌──────────────┐    ┌─────────────┐    ┌─────────────┐
│ SSH WebSocket│    │   VM Data   │    │   VM Files  │
│   Server     │    │  (JSON)     │    │  (QCOW2)    │
│ (Port 3001)  │    └─────────────┘    └─────────────┘
└──────────────┘
```

## Components Overview

### 1. Frontend (React + TypeScript)
- **Technology**: React 18, TypeScript, Bootstrap 5
- **Purpose**: Web-based user interface for VM management

### 2. Backend (Rust + Axum)
- **Port**: 8081
- **Technology**: Rust, Axum, Tokio
- **Purpose**: Core VM management API and WebSocket handling

### 3. Proxy (Rust + Axum)
- **Port**: 8080 (configurable)
- **Technology**: Rust, Axum, Reqwest
- **Purpose**: HTTP proxy that forwards API requests to the backend


### 4. SSH WebSocket Server (Node.js)
- **Port**: 3001
- **Technology**: Node.js, WebSocket, node-pty
- **Purpose**: SSH terminal access through WebSocket connections


## Port Configuration

| Component | Default Port | Environment Variable | Purpose |
|-----------|--------------|---------------------|---------|
| Frontend | 3000 | `PORT` | React development server |
| Backend | 8081 | `PORT` | VM management API |
| Proxy | 8080 | `PROXY_PORT` | HTTP request forwarding |
| SSH Server | 3001 | `PORT` | WebSocket SSH terminal |
