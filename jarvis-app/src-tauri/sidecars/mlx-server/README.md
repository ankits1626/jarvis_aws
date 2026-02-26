# MLX Sidecar Server

Python sidecar for local LLM inference using MLX on Apple Silicon.

## Requirements

- macOS with Apple Silicon (arm64)
- Python 3.10+
- MLX and dependencies (see requirements.txt)

## Installation

```bash
pip install -r requirements.txt
```

## Protocol

Communicates via NDJSON over stdin/stdout.

### Commands

#### check-availability
Check if MLX is available.

**Request:**
```json
{"command": "check-availability"}
```

**Response:**
```json
{"type": "response", "command": "check-availability", "available": true}
```

#### load-model
Load a model from disk.

**Request:**
```json
{"command": "load-model", "model_path": "/path/to/model"}
```

**Response:**
```json
{"type": "response", "command": "load-model", "success": true, "model_name": "model-name"}
```

#### generate-tags
Generate tags for content.

**Request:**
```json
{"command": "generate-tags", "content": "text to tag"}
```

**Response:**
```json
{"type": "response", "command": "generate-tags", "tags": ["tag1", "tag2"]}
```

#### summarize
Summarize content.

**Request:**
```json
{"command": "summarize", "content": "text to summarize"}
```

**Response:**
```json
{"type": "response", "command": "summarize", "summary": "summary text"}
```

#### download-model
Download a model from HuggingFace Hub.

**Request:**
```json
{"command": "download-model", "repo_id": "mlx-community/Qwen2.5-3B-Instruct-4bit", "destination": "/path/to/dest"}
```

**Progress:**
```json
{"type": "progress", "command": "download-model", "progress": 45.2, "downloaded_mb": 123.4}
```

**Response:**
```json
{"type": "response", "command": "download-model", "success": true, "destination": "/path/to/dest"}
```

#### model-info
Get information about loaded model.

**Request:**
```json
{"command": "model-info"}
```

**Response:**
```json
{"type": "response", "command": "model-info", "model_name": "model-name", "param_count": 3000000000}
```

#### shutdown
Gracefully shutdown the server.

**Request:**
```json
{"command": "shutdown"}
```

**Response:**
```json
{"type": "response", "command": "shutdown", "success": true}
```

## Error Handling

All errors return:
```json
{"type": "error", "command": "command-name", "error": "error message"}
```

## Testing

```bash
# Start server
python server.py

# Send commands via stdin
echo '{"command": "check-availability"}' | python server.py
```
