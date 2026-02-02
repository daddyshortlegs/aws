# Simple RAG Agent (Ollama)

A minimal Retrieval-Augmented Generation agent using local models via Ollama. No API keys required!

## Prerequisites

1. **Install Ollama**: https://ollama.ai
   ```bash
   # On macOS
   brew install ollama
   
   # Or download from https://ollama.ai
   ```

2. **Pull a model** (if not already done):
   ```bash
   ollama pull llama2
   # or
   ollama pull mistral
   # or
   ollama pull codellama
   ```

3. **Backend API running** (for API operations):
   - The backend service should be running at `http://127.0.0.1:8081` (default)
   - Or set `BACKEND_API_URL` environment variable to your backend URL

## Setup

1. Install Python dependencies:
   ```bash
   pip install -r requirements.txt
   ```

## Usage

Run the RAG agent as an HTTP server:

```bash
python agent.py
```

With custom host/port:

```bash
export RAG_PORT=8082
export RAG_HOST=0.0.0.0
python agent.py
```

The server starts on `http://0.0.0.0:8082` (default).

**API Documentation:**
- Swagger UI: `http://localhost:8082/docs`
- ReDoc: `http://localhost:8082/redoc`
- OpenAPI schema: `http://localhost:8082/openapi.json`

**Example API Calls:**

```bash
# Health check
curl http://localhost:8082/health

# Query endpoint
curl -X POST http://localhost:8082/query \
  -H "Content-Type: application/json" \
  -d '{"question": "list all VMs"}'

# Create a VM
curl -X POST http://localhost:8082/query \
  -H "Content-Type: application/json" \
  -d '{"question": "create a VM called test-vm"}'

# Delete a VM
curl -X POST http://localhost:8082/query \
  -H "Content-Type: application/json" \
  -d '{"question": "delete VM with id abc123"}'
```

## How It Works

### API Operations
1. **Intent Detection**: Uses LLM to detect if the query is an API operation
2. **Parameter Extraction**: Extracts operation parameters from natural language
3. **API Call**: Makes HTTP request to the backend API
4. **Response Formatting**: Formats the API response for the user


## Deployment

The RAG agent can be deployed as a systemd service using Ansible:

```bash
cd RAG
ansible-playbook -i ../inventory/hosts.yaml deploy-rag.yaml
```

This will:
1. Install Python, Ollama, and dependencies
2. Copy RAG files to the server
3. Create a virtual environment
4. Pull the llama2 model
5. Create and start a systemd service running the HTTP server on port 8082

The service will be available at `http://<server-ip>:8082`
