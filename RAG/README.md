# Simple RAG Agent (Ollama)

A minimal Retrieval-Augmented Generation agent using local models via Ollama. No API keys required!

Supports both:
- **Document queries**: Ask questions about documents in the knowledge base
- **API operations**: Perform VM management operations via natural language

Can run as:
- **CLI tool**: Interactive command-line interface
- **HTTP server**: FastAPI REST API with Swagger documentation

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

### CLI Mode (Interactive)

Run the agent in interactive mode:

```bash
python agent.py
```

Then you can:
- Ask questions about documents: "What is QEMU?"
- Create VMs: "create a VM called my-vm"
- List VMs: "list all VMs"
- Delete VMs: "delete VM with id abc123" or "delete VM called my-vm"

### Server Mode (HTTP API)

Run the agent as an HTTP server:

```bash
python agent.py server
```

Or with custom host/port:

```bash
export RAG_PORT=8082
export RAG_HOST=0.0.0.0
python agent.py server
```

The server will start on `http://0.0.0.0:8082` (default).

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

### Python API

```python
from RAG.agent import SimpleRAG

# Initialize agent (defaults to 'llama2' model)
agent = SimpleRAG(model="llama2", api_base_url="http://127.0.0.1:8081")

# Query documents or perform API operations
result = agent.query("create a VM called test-vm")
print(result["answer"])
```

## API Operations

The agent can perform the following VM management operations:

### Launch VM
- **Examples**:
  - "create a VM called test-vm"
  - "launch a new VM named production-server"
  - "start a VM called dev-env in us-west-2"
- **Parameters** (extracted automatically):
  - `name` (required): VM name
  - `instance_type` (optional, default: "t2.micro")
  - `region` (optional, default: "us-east-1")

### List VMs
- **Examples**:
  - "list all VMs"
  - "show me all virtual machines"
  - "what VMs are running?"

### Delete VM
- **Examples**:
  - "delete VM with id abc-123-def"
  - "remove the VM called test-vm"
  - "terminate VM test-vm"

## Available Models

You can use any Ollama model. Popular options:
- `llama2` - General purpose (default)
- `mistral` - Fast and efficient
- `codellama` - Code-focused
- `llama2:13b` - Larger, more capable version

Change the model in the `SimpleRAG()` constructor.

## How It Works

### Document Queries
1. **Document Loading**: Loads all `.txt` files from the `documents/` directory
2. **Embeddings**: Uses `sentence-transformers` to create embeddings (runs locally)
3. **Retrieval**: Finds the most relevant documents using cosine similarity
4. **Generation**: Uses Ollama to generate answers based on retrieved context

### API Operations
1. **Intent Detection**: Uses LLM to detect if the query is an API operation
2. **Parameter Extraction**: Extracts operation parameters from natural language
3. **API Call**: Makes HTTP request to the backend API
4. **Response Formatting**: Formats the API response for the user

## Configuration

Set environment variables to customize behavior:

```bash
# Backend API URL (default: http://127.0.0.1:8081)
export BACKEND_API_URL="http://localhost:8081"

# Server mode configuration
export RAG_PORT=8082        # Default: 8082
export RAG_HOST=0.0.0.0     # Default: 0.0.0.0

# Run in CLI mode
python agent.py

# Run in server mode
python agent.py server
```

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

## Examples

### Document Query
```bash
# Add a document
echo "Python is a programming language. It is known for its simplicity." > documents/python.txt

# Run the agent
python -m RAG.agent

# Ask: "What is Python?"
# Answer: Based on the context, Python is a programming language known for its simplicity.
```

### API Operation
```bash
# Run the agent (make sure backend is running)
python -m RAG.agent

# Ask: "create a VM called test-vm"
# Answer: ✅ VM launched successfully!
#         Instance ID: abc-123-def
#         Name: test-vm
#         SSH Port: 49152
#         Message: VM launch request received for test-vm in us-east-1
```
