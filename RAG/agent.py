"""
Simple RAG Agent using Ollama
The simplest possible RAG implementation using local models via Ollama.
Supports both document queries and API operations.
"""
import os
import subprocess
import json
from pathlib import Path
from typing import List, Dict, Optional
from sentence_transformers import SentenceTransformer
import numpy as np
import httpx

class SimpleRAG:
    def __init__(self, model: str = "llama2", documents_dir: str = "documents", api_base_url: str = "http://127.0.0.1:8081"):
        """
        Initialize the RAG agent.
        
        Args:
            model: Ollama model name (e.g., 'llama2', 'mistral', 'codellama')
            documents_dir: Directory containing text documents
            api_base_url: Base URL for the backend API
        """
        self.model = model
        self.documents_dir = Path(documents_dir)
        self.documents: List[str] = []
        self.embeddings: List[np.ndarray] = []
        self.api_base_url = api_base_url.rstrip('/')
        
        # Use a local embedding model (no API needed)
        print("Loading embedding model...")
        self.embedding_model = SentenceTransformer('all-MiniLM-L6-v2')
        print("Embedding model loaded!")
        
        # Check if Ollama is available
        self._check_ollama()
    
    def _check_ollama(self):
        """Check if Ollama is installed and the model is available."""
        try:
            result = subprocess.run(
                ['ollama', 'list'],
                capture_output=True,
                text=True,
                timeout=5
            )
            if result.returncode != 0:
                print("Warning: Ollama may not be installed or running.")
                print("Install from: https://ollama.ai")
                return
            
            # Check if model is available
            if self.model not in result.stdout:
                print(f"Model '{self.model}' not found. Pulling it now...")
                subprocess.run(['ollama', 'pull', self.model], check=True)
                print(f"Model '{self.model}' is ready!")
            else:
                print(f"Model '{self.model}' is available")
        except FileNotFoundError:
            print("Error: Ollama not found. Please install from https://ollama.ai")
            raise
        except subprocess.TimeoutExpired:
            print("Warning: Ollama may not be running. Start it with: ollama serve")
    
    def load_documents(self):
        """Load all .txt files from the documents directory."""
        if not self.documents_dir.exists():
            self.documents_dir.mkdir(parents=True, exist_ok=True)
            print(f"Created {self.documents_dir} directory. Add .txt files there.")
            return
        
        txt_files = list(self.documents_dir.glob("*.txt"))
        if not txt_files:
            print(f"No .txt files found in {self.documents_dir}")
            return
        
        self.documents = []
        for file_path in txt_files:
            with open(file_path, 'r', encoding='utf-8') as f:
                content = f.read().strip()
                if content:
                    self.documents.append(content)
                    print(f"Loaded: {file_path.name} ({len(content)} chars)")
        
        print(f"Loaded {len(self.documents)} documents")
    
    def create_embeddings(self):
        """Create embeddings for all documents."""
        if not self.documents:
            self.load_documents()
        
        if not self.documents:
            raise ValueError("No documents to embed. Add .txt files to the documents directory.")
        
        print("Creating embeddings...")
        # Create embeddings using sentence-transformers
        self.embeddings = self.embedding_model.encode(
            self.documents,
            convert_to_numpy=True,
            show_progress_bar=True
        )
        print(f"Created {len(self.embeddings)} embeddings")
    
    def _cosine_similarity(self, vec1: np.ndarray, vec2: np.ndarray) -> float:
        """Calculate cosine similarity between two vectors."""
        return np.dot(vec1, vec2) / (np.linalg.norm(vec1) * np.linalg.norm(vec2))
    
    def _find_relevant_documents(self, query: str, top_k: int = 3) -> List[str]:
        """Find the most relevant documents for a query."""
        if len(self.embeddings) == 0:
            raise ValueError("No embeddings found. Call create_embeddings() first.")
        
        # Embed the query
        query_embedding = self.embedding_model.encode([query], convert_to_numpy=True)[0]
        
        # Calculate similarities
        # self.embeddings is a 2D numpy array, iterate over rows
        similarities = [
            self._cosine_similarity(query_embedding, doc_emb)
            for doc_emb in self.embeddings
        ]
        
        # Get top k documents
        top_indices = np.argsort(similarities)[-top_k:][::-1]
        
        return [self.documents[i] for i in top_indices]
    
    def _query_ollama(self, prompt: str) -> str:
        """Query Ollama with a prompt."""
        try:
            result = subprocess.run(
                ['ollama', 'run', self.model, prompt],
                capture_output=True,
                text=True,
                timeout=60
            )
            
            if result.returncode != 0:
                return f"Error: {result.stderr}"
            
            return result.stdout.strip()
        except subprocess.TimeoutExpired:
            return "Error: Request timed out"
        except Exception as e:
            return f"Error: {str(e)}"
    
    def _detect_api_operation(self, question: str) -> Optional[Dict[str, any]]:
        """
        Use LLM to detect if the question is an API operation and extract parameters.
        Returns None if not an API operation, or a dict with operation details.
        """
        prompt = f"""Analyze the following user question and determine if it's a request to perform an API operation on a VM management system.

Available API operations:
1. launch-vm: Create/launch a new VM. Requires: name (required), instance_type (optional, default: "t2.micro"), region (optional, default: "us-east-1")
2. list-vms: List all VMs. No parameters needed.
3. delete-vm: Delete a VM. Requires: id (the VM instance ID) or name (to find the ID first)

User question: "{question}"

Respond with ONLY a JSON object in this exact format (no other text):
- If it's an API operation: {{"operation": "launch-vm"|"list-vms"|"delete-vm", "params": {{"name": "...", "instance_type": "...", "region": "...", "id": "..."}}}}
- If it's NOT an API operation: {{"operation": null}}

Extract parameters from the question. If a parameter is not mentioned, use defaults:
- instance_type: "t2.micro"
- region: "us-east-1"

Examples:
- "create a VM called test-vm" -> {{"operation": "launch-vm", "params": {{"name": "test-vm", "instance_type": "t2.micro", "region": "us-east-1"}}}}
- "list all VMs" -> {{"operation": "list-vms", "params": {{}}}}
- "delete the VM with id abc123" -> {{"operation": "delete-vm", "params": {{"id": "abc123"}}}}
- "what is a VM?" -> {{"operation": null}}

JSON response:"""
        
        response = self._query_ollama(prompt)
        
        # Try to extract JSON from response
        try:
            # Look for JSON in the response
            start_idx = response.find('{')
            end_idx = response.rfind('}') + 1
            if start_idx >= 0 and end_idx > start_idx:
                json_str = response[start_idx:end_idx]
                result = json.loads(json_str)
                if result.get("operation"):
                    return result
        except (json.JSONDecodeError, ValueError) as e:
            print(f"Warning: Could not parse API operation detection: {e}")
        
        return None
    
    def _call_api(self, operation: str, params: Dict) -> Dict[str, any]:
        """Call the backend API with the given operation and parameters."""
        try:
            with httpx.Client(timeout=30.0) as client:
                if operation == "launch-vm":
                    payload = {
                        "name": params.get("name", "unnamed-vm"),
                        "instance_type": params.get("instance_type", "t2.micro"),
                        "region": params.get("region", "us-east-1")
                    }
                    response = client.post(
                        f"{self.api_base_url}/launch-vm",
                        json=payload
                    )
                    response.raise_for_status()
                    return {
                        "success": True,
                        "operation": "launch-vm",
                        "data": response.json()
                    }
                
                elif operation == "list-vms":
                    response = client.get(f"{self.api_base_url}/list-vms")
                    response.raise_for_status()
                    data = response.json()
                    # Backend returns a list directly, or a dict with "vms" key
                    # Normalize to always have a "vms" key
                    if isinstance(data, list):
                        data = {"vms": data}
                    return {
                        "success": True,
                        "operation": "list-vms",
                        "data": data
                    }
                
                elif operation == "delete-vm":
                    # If name is provided, first list VMs to find the ID
                    vm_id = params.get("id")
                    if not vm_id and params.get("name"):
                        list_response = client.get(f"{self.api_base_url}/list-vms")
                        list_response.raise_for_status()
                        vms_data = list_response.json()
                        # Backend returns a list directly, or a dict with "vms" key
                        if isinstance(vms_data, list):
                            vms = vms_data
                        else:
                            vms = vms_data.get("vms", [])
                        # Find VM by name
                        for vm in vms:
                            if vm.get("name") == params["name"]:
                                vm_id = vm.get("id")
                                break
                        
                        if not vm_id:
                            return {
                                "success": False,
                                "operation": "delete-vm",
                                "error": f"VM with name '{params['name']}' not found"
                            }
                    
                    if not vm_id:
                        return {
                            "success": False,
                            "operation": "delete-vm",
                            "error": "VM ID or name is required"
                        }
                    
                    response = client.delete(
                        f"{self.api_base_url}/delete-vm",
                        json={"id": vm_id}
                    )
                    response.raise_for_status()
                    return {
                        "success": True,
                        "operation": "delete-vm",
                        "data": {"message": response.text}
                    }
                
                else:
                    return {
                        "success": False,
                        "error": f"Unknown operation: {operation}"
                    }
        
        except httpx.HTTPStatusError as e:
            return {
                "success": False,
                "operation": operation,
                "error": f"API error: {e.response.status_code} - {e.response.text}"
            }
        except httpx.RequestError as e:
            return {
                "success": False,
                "operation": operation,
                "error": f"Connection error: {str(e)}. Is the backend running at {self.api_base_url}?"
            }
        except Exception as e:
            return {
                "success": False,
                "operation": operation,
                "error": f"Unexpected error: {str(e)}"
            }
    
    def query(self, question: str, top_k: int = 3) -> Dict[str, any]:
        """
        Query the RAG agent. Can handle both API operations and document queries.
        
        Args:
            question: The question to ask or API operation to perform
            top_k: Number of relevant documents to retrieve (for document queries)
            
        Returns:
            Dictionary with 'answer', 'context', 'api_result' keys
        """
        # First, check if this is an API operation
        print("Detecting if this is an API operation...")
        api_op = self._detect_api_operation(question)
        
        if api_op and api_op.get("operation"):
            # This is an API operation
            operation = api_op["operation"]
            params = api_op.get("params", {})
            
            print(f"Detected API operation: {operation} with params: {params}")
            api_result = self._call_api(operation, params)
            
            # Format the result for the user
            if api_result["success"]:
                if operation == "launch-vm":
                    data = api_result["data"]
                    answer = f"✅ VM launched successfully!\n"
                    answer += f"Instance ID: {data.get('instance_id', 'N/A')}\n"
                    answer += f"Name: {params.get('name', 'N/A')}\n"
                    answer += f"SSH Port: {data.get('ssh_port', 'N/A')}\n"
                    answer += f"Message: {data.get('message', 'N/A')}"
                elif operation == "list-vms":
                    data = api_result["data"]
                    vms = data.get("vms", [])
                    if vms:
                        answer = f"📋 Found {len(vms)} VM(s):\n\n"
                        for vm in vms:
                            answer += f"  • {vm.get('name', 'N/A')} (ID: {vm.get('id', 'N/A')}, SSH Port: {vm.get('ssh_port', 'N/A')}, PID: {vm.get('pid', 'N/A')})\n"
                    else:
                        answer = "📋 No VMs found."
                elif operation == "delete-vm":
                    answer = f"✅ VM deleted successfully!\n{api_result.get('data', {}).get('message', 'VM removed')}"
            else:
                answer = f"❌ Error performing {operation}: {api_result.get('error', 'Unknown error')}"
            
            return {
                "answer": answer,
                "api_result": api_result,
                "is_api_operation": True
            }
        
        # This is a document query
        if len(self.embeddings) == 0:
            self.create_embeddings()
        
        # Find relevant documents
        relevant_docs = self._find_relevant_documents(question, top_k)
        context = "\n\n---\n\n".join(relevant_docs)
        
        # Build prompt for Ollama
        prompt = f"""Answer the following question using only the information provided in the context below.

If the context doesn't contain enough information to answer the question, say so clearly.

Context:
{context}

Question: {question}

Answer:"""
        
        # Query Ollama
        print(f"Querying {self.model} for document-based answer...")
        answer = self._query_ollama(prompt)
        
        return {
            "answer": answer,
            "context": context[:500] + "..." if len(context) > 500 else context,  # Truncated for display
            "is_api_operation": False
        }


def main():
    """Example usage."""
    import sys
    
    # Allow API URL to be set via environment variable
    api_url = os.getenv("BACKEND_API_URL", "http://127.0.0.1:8081")
    
    # Initialize agent (you can change 'llama2' to 'mistral', 'codellama', etc.)
    agent = SimpleRAG(model="llama2", api_base_url=api_url)
    
    # Load and embed documents (only if documents exist)
    try:
        agent.create_embeddings()
    except ValueError:
        print("No documents found. Agent will work for API operations only.")
    
    print("\n" + "="*60)
    print("RAG Agent Ready!")
    print("="*60)
    print("You can:")
    print("  • Ask questions about documents")
    print("  • Create VMs: 'create a VM called my-vm'")
    print("  • List VMs: 'list all VMs'")
    print("  • Delete VMs: 'delete VM with id abc123' or 'delete VM called my-vm'")
    print("="*60)
    
    # Query the agent
    while True:
        question = input("\nAsk a question or perform an operation (or 'quit' to exit): ")
        if question.lower() in ['quit', 'exit', 'q']:
            break
        
        result = agent.query(question)
        print(f"\n{result['answer']}")
        
        if not result.get('is_api_operation') and result.get('context'):
            print(f"\n(Used context: {result['context'][:200]}...)")


if __name__ == "__main__":
    main()
