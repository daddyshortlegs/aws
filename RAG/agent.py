"""
Simple RAG Agent using Ollama
The simplest possible RAG implementation using local models via Ollama.
Supports both document queries and API operations.
"""
import os
import subprocess
import json

from ollama import Client
from ollama import ChatResponse

from pathlib import Path
from typing import List, Dict, Optional
from sentence_transformers import SentenceTransformer
import numpy as np
import httpx


class SimpleRAG:
    def __init__(self, model: str = "llama2", api_base_url: str = "http://127.0.0.1:8081"):
        """
        Args:
            model: Ollama model name (e.g., 'llama2', 'mistral', 'codellama')
            api_base_url: Base URL for the backend API
        """
        self.model = model

        self.client = Client(
            host="http://localhost:11434"
        )

        self.api_base_url = api_base_url.rstrip('/')
  
    def extractJsonFromResponse(self, response: str) -> Optional[Dict[str, any]]:
        try:
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

    def _detect_api_operation(self, question: str) -> Optional[Dict[str, any]]:

        messages = [
        {
            'role': 'system',
            'content': f"""You are a JSON-only API. Analyze the user question and determine if it's a request to perform an API operation on a VM management system.

        Available API operations:
        1. launch-vm: Create/launch a new VM. Requires: name (required), instance_type (optional, default: "t2.micro"), region (optional, default: "us-east-1")
        2. list-vms: List all VMs. No parameters needed.
        3. delete-vm: Delete a VM. Requires: id (the VM instance ID) or name (to find the ID first)

        CRITICAL: Respond with ONLY valid JSON. No markdown, no explanations, no code blocks, no extra text. Just the raw JSON object.

        Format:
        - If it's an API operation: {{"operation": "launch-vm"|"list-vms"|"delete-vm", "params": {{"name": "...", "instance_type": "...", "region": "...", "id": "..."}}}}
        - If it's NOT an API operation: {{"operation": null}}

        Extract parameters from the question. If a parameter is not mentioned, use defaults:
        - instance_type: "t2.micro"
        - region: "us-east-1"

        If a name is used rather than an ID in the operations, first list all VMs to find the ID.

        Examples:
        - "create a VM called test-vm" -> {{"operation": "launch-vm", "params": {{"name": "test-vm", "instance_type": "t2.micro", "region": "us-east-1"}}}}
        - "list all VMs" -> {{"operation": "list-vms", "params": {{}}}}
        - "delete the VM with id abc123" -> {{"operation": "delete-vm", "params": {{"id": "abc123"}}}}
        - "what is a VM?" -> {{"operation": null}}"""
        },
        {
            'role': 'user',
            'content': question,
        },
        ]


        message: ChatResponse = self.client.chat(self.model, messages=messages)
        
        response = message.message.content
        print("response: ", response)

        result = self.extractJsonFromResponse(response)
        if result:
            return result
        
        return None

    def _call_api(self, operation: str, params: Dict) -> Dict[str, any]:
        try:
            with httpx.Client(timeout=30.0) as client:
                if operation == "launch-vm":
                    return self._launch_vm(params, client)
                elif operation == "list-vms":
                    return self._list_vms(client)
                elif operation == "delete-vm":
                    return self._delete_vm(params, client)
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
    
    
    def _launch_vm(self, params: Dict, client: httpx.Client) -> Dict[str, any]:
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

    def _list_vms(self, client: httpx.Client) -> Dict[str, any]:
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

    def _delete_vm(self, params: Dict, client: httpx.Client) -> Dict[str, any]:
        # If name is provided, first list VMs to find the ID
        vm_id = params.get("id")
        print("vm_id: ", vm_id)
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

        print("deleting vm with id: ", vm_id)
        response = client.request(
            "DELETE",
            f"{self.api_base_url}/delete-vm",
            json={"id": vm_id}
        )
        response.raise_for_status()
        return {
            "success": True,
            "operation": "delete-vm",
            "data": {"message": response.text}
        }

    def query(self, question: str) -> Dict[str, any]:
        """
        Query the RAG agent. Can handle both API operations and document queries.
        
        Args:
            question: The question to ask or API operation to perform
            
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
        
        # Not an API operation - use Ollama to answer the question
        print(f"Querying {self.model} for general question...")

        messages = [
            {
                'role': 'user',
                'content': question
            }
        ]
        response: ChatResponse = self.client.chat(self.model, messages=messages)
        
        return {
            "answer": response.message.content,
            "is_api_operation": False
        }

def main():
    import sys
    
    api_url = os.getenv("BACKEND_API_URL", "http://127.0.0.1:8081")
    agent = SimpleRAG(model="llama2", api_base_url=api_url)
    
    print("\n" + "="*60)
    print("AWS Agent Ready!")
    print("="*60)
    print("You can:")
    print("  • Ask questions about documents")
    print("  • Create VMs: 'create a VM called my-vm'")
    print("  • List VMs: 'list all VMs'")
    print("  • Delete VMs: 'delete VM with id abc123' or 'delete VM called my-vm'")
    print("="*60)
    
    while True:
        question = input("\nAsk a question or perform an operation (or 'quit' to exit): ")
        if question.lower() in ['quit', 'exit', 'q']:
            break
        
        result = agent.query(question)
        if result and result.get('answer'):
            print(f"\n{result['answer']}")
        else:
            print("\n❌ Error: No response from agent")


if __name__ == "__main__":
    main()
