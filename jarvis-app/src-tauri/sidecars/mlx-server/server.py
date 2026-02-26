#!/usr/bin/env python3
"""
MLX Sidecar Server for Jarvis
Provides local LLM inference using MLX on Apple Silicon.
Communicates via NDJSON over stdin/stdout.
"""

import sys
import json
import platform
import re
import traceback
from typing import Optional, Dict, Any, List

# Check platform
if platform.machine() != "arm64":
    print(json.dumps({
        "type": "error",
        "error": "MLX requires Apple Silicon (arm64)"
    }), file=sys.stdout, flush=True)
    sys.exit(1)

try:
    import mlx.core as mx
    from mlx_lm import load, generate
    from huggingface_hub import snapshot_download
except ImportError as e:
    print(json.dumps({
        "type": "error",
        "error": f"Failed to import MLX dependencies: {str(e)}"
    }), file=sys.stdout, flush=True)
    sys.exit(1)


class MLXServer:
    def __init__(self):
        self.model = None
        self.tokenizer = None
        self.model_name = None
        
    def check_availability(self) -> Dict[str, Any]:
        """Check if MLX is available and working."""
        try:
            # Simple MLX operation to verify it works
            _ = mx.array([1, 2, 3])
            return {
                "type": "response",
                "command": "check-availability",
                "available": True
            }
        except Exception as e:
            return {
                "type": "response",
                "command": "check-availability",
                "available": False,
                "error": str(e)
            }
    
    def load_model(self, model_path: str) -> Dict[str, Any]:
        """Load an MLX model from disk."""
        try:
            self.model, self.tokenizer = load(model_path)
            self.model_name = model_path.split("/")[-1]
            
            return {
                "type": "response",
                "command": "load-model",
                "success": True,
                "model_name": self.model_name
            }
        except Exception as e:
            return {
                "type": "error",
                "command": "load-model",
                "error": str(e)
            }
    
    def _parse_tags(self, response: str) -> List[str]:
        """Parse tags from model response, handling various output formats."""
        text = response.strip()

        # Try to extract a JSON array from the response (model may add extra text around it)
        match = re.search(r'\[.*?\]', text, re.DOTALL)
        if match:
            try:
                tags = json.loads(match.group())
                if isinstance(tags, list):
                    return [str(t).strip() for t in tags if str(t).strip()]
            except json.JSONDecodeError:
                pass

        # Fallback: strip brackets, split by commas
        text = text.strip('[]')
        tags = [t.strip().strip('"\'') for t in text.replace('\n', ',').split(',')]
        return [t for t in tags if t]

    def generate_tags(self, content: str) -> Dict[str, Any]:
        """Generate tags for content."""
        if self.model is None:
            return {
                "type": "error",
                "command": "generate-tags",
                "error": "No model loaded"
            }
        
        try:
            prompt = f"""Generate 3-5 relevant tags for this content. Return ONLY a JSON array of strings, nothing else.

Content: {content[:2000]}

Tags:"""
            
            # Append /no_think suffix to model name for faster inference
            response = generate(
                self.model,
                self.tokenizer,
                prompt=prompt,
                max_tokens=200,
                verbose=False
            )
            
            # Parse the response as JSON array
            tags = self._parse_tags(response)
            
            return {
                "type": "response",
                "command": "generate-tags",
                "tags": tags[:5]  # Max 5 tags
            }
        except Exception as e:
            return {
                "type": "error",
                "command": "generate-tags",
                "error": str(e)
            }
    
    def summarize(self, content: str) -> Dict[str, Any]:
        """Summarize content."""
        if self.model is None:
            return {
                "type": "error",
                "command": "summarize",
                "error": "No model loaded"
            }
        
        try:
            prompt = f"""Summarize this content in 2-3 sentences. Be concise and factual.

Content: {content[:2000]}

Summary:"""
            
            # Append /no_think suffix to model name for faster inference
            response = generate(
                self.model,
                self.tokenizer,
                prompt=prompt,
                max_tokens=150,
                verbose=False
            )
            
            summary = response.strip()
            
            return {
                "type": "response",
                "command": "summarize",
                "summary": summary
            }
        except Exception as e:
            return {
                "type": "error",
                "command": "summarize",
                "error": str(e)
            }
    
    def download_model(self, repo_id: str, destination: str) -> None:
        """Download a model from HuggingFace Hub with progress reporting."""
        try:
            def progress_callback(progress_info):
                # Emit progress updates
                if hasattr(progress_info, 'downloaded') and hasattr(progress_info, 'total'):
                    percentage = (progress_info.downloaded / progress_info.total) * 100
                    downloaded_mb = progress_info.downloaded / (1024 * 1024)
                    
                    print(json.dumps({
                        "type": "progress",
                        "command": "download-model",
                        "progress": round(percentage, 2),
                        "downloaded_mb": round(downloaded_mb, 2)
                    }), file=sys.stdout, flush=True)
            
            # Download the model
            snapshot_download(
                repo_id=repo_id,
                local_dir=destination,
                local_dir_use_symlinks=False
            )
            
            # Emit completion
            print(json.dumps({
                "type": "response",
                "command": "download-model",
                "success": True,
                "destination": destination
            }), file=sys.stdout, flush=True)
            
        except Exception as e:
            print(json.dumps({
                "type": "error",
                "command": "download-model",
                "error": str(e)
            }), file=sys.stdout, flush=True)
    
    def model_info(self) -> Dict[str, Any]:
        """Get information about the loaded model."""
        if self.model is None:
            return {
                "type": "error",
                "command": "model-info",
                "error": "No model loaded"
            }
        
        try:
            # Estimate parameter count (rough approximation)
            param_count = sum(p.size for p in self.model.parameters())
            
            return {
                "type": "response",
                "command": "model-info",
                "model_name": self.model_name,
                "param_count": param_count
            }
        except Exception as e:
            return {
                "type": "error",
                "command": "model-info",
                "error": str(e)
            }
    
    def handle_command(self, command_data: Dict[str, Any]) -> Optional[Dict[str, Any]]:
        """Route commands to appropriate handlers."""
        command = command_data.get("command")
        
        if command == "check-availability":
            return self.check_availability()
        
        elif command == "load-model":
            model_path = command_data.get("model_path")
            if not model_path:
                return {"type": "error", "command": command, "error": "Missing model_path"}
            return self.load_model(model_path)
        
        elif command == "generate-tags":
            content = command_data.get("content")
            if not content:
                return {"type": "error", "command": command, "error": "Missing content"}
            return self.generate_tags(content)
        
        elif command == "summarize":
            content = command_data.get("content")
            if not content:
                return {"type": "error", "command": command, "error": "Missing content"}
            return self.summarize(content)
        
        elif command == "download-model":
            repo_id = command_data.get("repo_id")
            destination = command_data.get("destination")
            if not repo_id or not destination:
                return {"type": "error", "command": command, "error": "Missing repo_id or destination"}
            # Download runs synchronously and emits its own responses
            self.download_model(repo_id, destination)
            return None  # Already emitted response
        
        elif command == "model-info":
            return self.model_info()
        
        elif command == "shutdown":
            return {"type": "response", "command": "shutdown", "success": True}
        
        else:
            return {"type": "error", "error": f"Unknown command: {command}"}
    
    def run(self):
        """Main event loop: read NDJSON from stdin, write to stdout."""
        for line in sys.stdin:
            line = line.strip()
            if not line:
                continue
            
            try:
                command_data = json.loads(line)
                response = self.handle_command(command_data)
                
                if response:
                    print(json.dumps(response), file=sys.stdout, flush=True)
                
                # Exit on shutdown command
                if command_data.get("command") == "shutdown":
                    break
                    
            except json.JSONDecodeError as e:
                print(json.dumps({
                    "type": "error",
                    "error": f"Invalid JSON: {str(e)}"
                }), file=sys.stdout, flush=True)
            except Exception as e:
                print(json.dumps({
                    "type": "error",
                    "error": f"Unexpected error: {str(e)}",
                    "traceback": traceback.format_exc()
                }), file=sys.stdout, flush=True)


if __name__ == "__main__":
    server = MLXServer()
    server.run()
