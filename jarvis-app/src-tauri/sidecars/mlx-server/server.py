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
    from mlx_lm import load
    from mlx_lm import generate as mlx_lm_generate
    from huggingface_hub import snapshot_download
except ImportError as e:
    print(json.dumps({
        "type": "error",
        "error": f"Failed to import MLX dependencies: {str(e)}"
    }), file=sys.stdout, flush=True)
    sys.exit(1)

# Compatibility shim: mlx-lm >= 0.30 renamed get_model_path -> hf_repo_to_path
# mlx-lm-omni 0.1.3 imports get_model_path, so we provide it before importing mlx_lm_omni.
# We also add local path support since hf_repo_to_path only handles HF repo IDs.
import os
from pathlib import Path as _Path
import mlx_lm.utils as _mlx_lm_utils
if not hasattr(_mlx_lm_utils, 'get_model_path'):
    _original_hf_fn = getattr(_mlx_lm_utils, 'hf_repo_to_path', None)
    def _get_model_path(path_or_hf_repo: str) -> _Path:
        # Local directory path — return directly
        if os.path.isdir(path_or_hf_repo):
            return _Path(path_or_hf_repo)
        # HF repo ID — resolve via snapshot_download
        if _original_hf_fn:
            return _original_hf_fn(path_or_hf_repo)
        raise ValueError(f"Path not found and no HF resolver available: {path_or_hf_repo}")
    _mlx_lm_utils.get_model_path = _get_model_path

# Try to import mlx_lm_omni for multimodal support
# This is optional - only needed when using multimodal models
# IMPORTANT: mlx_lm.load/generate do NOT support multimodal architectures
# We must use mlx_lm_omni.load for loading and mlx_lm_omni.generate for inference
try:
    from mlx_lm_omni import load as mlx_omni_load
    from mlx_lm_omni import generate as mlx_omni_generate
    OMNI_AVAILABLE = True
except ImportError:
    mlx_omni_load = None
    mlx_omni_generate = None
    OMNI_AVAILABLE = False


def apply_runtime_patches():
    """Apply runtime patches for mlx-lm-omni v0.1.3 bugs.
    
    These patches fix six critical bugs in mlx-lm-omni <= 0.1.3:
    1. AudioTower reshape bug (causes failure on audio > 15s) - RUNTIME PATCH
    2. AudioMel precision loss (float16 → float32) - RUNTIME PATCH
    3. ExtendedQuantizedEmbedding kwargs compatibility - RUNTIME PATCH
    4. Model attribute delegation to tokenizer - RUNTIME PATCH
    5. Prefill chunking bug (causes IndexError on audio > 30s) - CALL-SITE FIX (prefill_step_size=32768)
    6. 7B model conv weight layout detection - RUNTIME PATCH
    
    Bugs #1-4 and #6 are fixed via runtime monkey-patching in this function.
    Bug #5 is fixed at call-site in generate_transcript() via prefill_step_size parameter.
    
    Patches are version-gated and automatically disabled for versions > 0.1.3.
    """
    try:
        import mlx_lm_omni
        from packaging import version

        # Only apply patches for versions <= 0.1.3
        omni_version = getattr(mlx_lm_omni, '__version__', '0.1.3')
        if version.parse(omni_version) > version.parse("0.1.3"):
            print(f"MLX: mlx-lm-omni version {omni_version} > 0.1.3, skipping patches",
                  file=sys.stderr, flush=True)
            return
        
        patches_applied = []
        
        # Patch 1: AudioTower.__call__ - move reshape AFTER transformer loop
        # Bug: mlx-lm-omni reshapes all chunks into a single sequence BEFORE the transformer,
        # causing full cross-chunk attention with repeated positional embeddings.
        # Fix: each chunk should process independently through the transformer, then merge.
        try:
            from mlx_lm_omni.models.qwen_omni.audio_tower import AudioTower
            import mlx.nn as nn
            import math as _math

            def patched_audio_tower_call(self, audio_mel: mx.array) -> mx.array:
                x_size = audio_mel.shape[1] // 2
                if audio_mel.shape[1] % (self.n_window * 2) != 0:
                    last_chunk_size = audio_mel.shape[1] % (self.n_window * 2)
                    audio_mel = mx.pad(audio_mel, pad_width=[(0, 0), (0, self.n_window * 2 - last_chunk_size)], mode="constant", constant_values=0)
                else:
                    last_chunk_size = self.n_window

                chunks_count = _math.floor(audio_mel.shape[1] / (self.n_window * 2))
                chunks = mx.reshape(audio_mel, (audio_mel.shape[0], chunks_count, self.n_window * 2))
                chunks = mx.transpose(chunks, (1, 2, 0))

                x = nn.gelu(self.conv1(chunks))

                if last_chunk_size != self.n_window:
                    x[chunks_count - 1, last_chunk_size:, :] = 0

                x = nn.gelu(self.conv2(x))

                embed_pos = mx.expand_dims(self._positional_embedding[:x.shape[1]], axis=0)
                x = x + embed_pos

                # Transformer: each chunk is a batch element → independent attention (correct!)
                for block in self.layers:
                    x, _, _ = block(x)

                # Reshape AFTER transformer so chunks were processed independently
                x = mx.reshape(x, (1, x.shape[0] * x.shape[1], x.shape[2]))[:, :x_size, :]

                x = self.avg_pooler(x)
                x = self.ln_post(x)
                x = self.proj(x)
                return x

            AudioTower.__call__ = patched_audio_tower_call
            patches_applied.append("AudioTower.__call__")
        except Exception as e:
            print(f"MLX: Warning - failed to patch AudioTower: {e}", file=sys.stderr, flush=True)

        # Patch 2: AudioMel - use float32 for precision (mel_filters is numpy, not MLX)
        try:
            import numpy as np
            from mlx_lm_omni.audio_mel import AudioMel
            original_init = AudioMel.__init__

            def patched_init(self, *args, **kwargs):
                original_init(self, *args, **kwargs)
                # Convert mel_filters from float16 to float32 for precision
                if hasattr(self, 'mel_filters'):
                    self.mel_filters = self.mel_filters.astype(np.float32)

            AudioMel.__init__ = patched_init
            patches_applied.append("AudioMel.float32")
        except Exception as e:
            print(f"MLX: Warning - failed to patch AudioMel: {e}", file=sys.stderr, flush=True)
        
        # Patch 3: ExtendedEmbedding.to_quantized - accept **kwargs (MLX 0.30+ passes 'mode')
        try:
            from mlx_lm_omni.tokenizer import ExtendedEmbedding
            original_ee_to_quantized = ExtendedEmbedding.to_quantized

            def patched_ee_to_quantized(self, group_size: int = 64, bits: int = 4, **kwargs):
                # Ignore extra kwargs (e.g. 'mode' from MLX 0.30+), call original
                return original_ee_to_quantized(self, group_size, bits)

            ExtendedEmbedding.to_quantized = patched_ee_to_quantized
            patches_applied.append("ExtendedEmbedding.to_quantized")
        except Exception as e:
            print(f"MLX: Warning - failed to patch ExtendedEmbedding: {e}", file=sys.stderr, flush=True)

        # Patch 4: TokenizerWithAudio - mlx_lm compatibility
        # mlx_lm.generate wraps tokenizer in TokenizerWrapper which accesses
        # chat_template, get_vocab, bos_token, etc. TokenizerWithAudio doesn't
        # expose these, but its inner _tokenizer does. Also, TokenizerWrapper
        # delegates encode() back, passing add_special_tokens kwarg that
        # TokenizerWithAudio.encode() doesn't accept.
        try:
            from mlx_lm_omni.models.qwen_omni.model import TokenizerWithAudio

            # __getattr__ fallback: delegate missing attributes to inner _tokenizer
            def _twa_getattr(self, name):
                return getattr(self._tokenizer, name)
            TokenizerWithAudio.__getattr__ = _twa_getattr

            # Wrap encode to accept and ignore extra kwargs (e.g. add_special_tokens)
            _original_encode = TokenizerWithAudio.encode
            def _twa_encode(self, text, **kwargs):
                return _original_encode(self, text)
            TokenizerWithAudio.encode = _twa_encode

            patches_applied.append("TokenizerWithAudio.compat")
        except Exception as e:
            print(f"MLX: Warning - failed to patch TokenizerWithAudio: {e}", file=sys.stderr, flush=True)
        
        # Patch 5: 7B model conv weight layout — handled in load_model() AFTER
        # weights are loaded from disk. An __init__ patch cannot work because
        # load() replaces weights after construction.
        patches_applied.append("AudioEncoder.conv_layout (post-load)")
        
        print(f"MLX: Applied patches for mlx-lm-omni {omni_version}: {', '.join(patches_applied)}", 
              file=sys.stderr, flush=True)
        
    except ImportError:
        # mlx-lm-omni not installed - this is fine, patches only needed when using multimodal models
        pass
    except Exception as e:
        print(f"MLX: Warning - failed to apply some patches: {e}", file=sys.stderr, flush=True)


class MLXServer:
    def __init__(self):
        self.model = None
        self.tokenizer = None
        self.model_name = None
        self.capabilities = []  # Track model capabilities from catalog
        
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
    
    def load_model(self, model_path: str, capabilities: Optional[List[str]] = None) -> Dict[str, Any]:
        """Load an MLX model from disk.
        
        Args:
            model_path: Path to the model directory
            capabilities: List of capabilities from catalog (e.g., ["audio", "text"])
                         If not provided, defaults to ["text"] for backward compatibility
        """
        try:
            # Store capabilities first so we know which loader to use
            # Default to ["text"] if not provided (backward compatibility)
            self.capabilities = capabilities if capabilities is not None else ["text"]

            # Use the appropriate loader based on capabilities
            # Multimodal models (audio+text) require mlx_lm_omni.load
            # Text-only models use standard mlx_lm.load
            if "audio" in self.capabilities:
                if not OMNI_AVAILABLE or mlx_omni_load is None:
                    return {
                        "type": "error",
                        "command": "load-model",
                        "error": "mlx-lm-omni not installed. Install it to load multimodal models."
                    }
                self.model, self.tokenizer = mlx_omni_load(model_path)

                # Fix conv weight layout after loading (7B weights ship in PyTorch layout)
                # PyTorch: (out_channels, in_channels, kernel_size)
                # MLX:     (out_channels, kernel_size, in_channels)
                # Must happen AFTER load() since __init__ sees default weights, not loaded ones.
                try:
                    at = self.model.thinker.audio_tower
                    if hasattr(at, 'conv1') and at.conv1.weight.shape[1] != 3:
                        at.conv1.weight = mx.swapaxes(at.conv1.weight, 1, 2)
                        at.conv2.weight = mx.swapaxes(at.conv2.weight, 1, 2)
                        print("MLX: Fixed conv weight layout (PyTorch → MLX) after load",
                              file=sys.stderr, flush=True)
                except Exception as e:
                    print(f"MLX: Conv weight check skipped: {e}", file=sys.stderr, flush=True)
            else:
                self.model, self.tokenizer = load(model_path)

            self.model_name = model_path.split("/")[-1]
            
            return {
                "type": "response",
                "command": "load-model",
                "success": True,
                "model_name": self.model_name,
                "capabilities": self.capabilities
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
            response = mlx_lm_generate(
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
            response = mlx_lm_generate(
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
    
    def chat(self, messages: List[Dict[str, str]]) -> Dict[str, Any]:
        """Handle multi-turn chat conversation.
        
        Args:
            messages: Array of {role, content} objects where role is "system", "user", or "assistant"
            
        Returns:
            Dict with type, command, and response fields
        """
        if self.model is None:
            return {
                "type": "error",
                "command": "chat",
                "error": "No model loaded"
            }
        
        if not messages:
            return {
                "type": "error",
                "command": "chat",
                "error": "No messages provided"
            }
        
        try:
            from mlx_lm.sample_utils import make_logits_processors, make_sampler

            # Build conversation from messages array
            conversation = [{"role": m["role"], "content": m["content"]} for m in messages]

            # Apply chat template and generate
            token_ids = self.tokenizer.apply_chat_template(
                conversation,
                add_generation_prompt=True
            )

            # Repetition penalty prevents degenerate looping responses
            logits_processors = make_logits_processors(
                repetition_penalty=1.2,
                repetition_context_size=64,
            )
            sampler = make_sampler(temp=0.7, top_p=0.9)

            response_text = mlx_lm_generate(
                self.model,
                self.tokenizer,
                prompt=token_ids,
                max_tokens=512,
                logits_processors=logits_processors,
                sampler=sampler,
                verbose=False
            )

            return {
                "type": "response",
                "command": "chat",
                "response": response_text.strip()
            }
        except Exception as e:
            return {
                "type": "error",
                "command": "chat",
                "error": str(e)
            }
    
    def generate_transcript(self, audio_path: str) -> Dict[str, Any]:
        """Generate transcript from audio file.
        
        Args:
            audio_path: Path to audio file (.wav or .pcm format)
            
        Returns:
            Dict with type, command, language, and transcript fields
        """
        if self.model is None:
            return {
                "type": "error",
                "command": "generate-transcript",
                "error": "No model loaded"
            }
        
        # Check capabilities from catalog, not model introspection
        if "audio" not in self.capabilities:
            return {
                "type": "error",
                "command": "generate-transcript",
                "error": "Loaded model does not support audio transcription"
            }
        
        # Check if mlx_lm_omni.generate is available
        if not OMNI_AVAILABLE or mlx_omni_generate is None:
            return {
                "type": "error",
                "command": "generate-transcript",
                "error": "mlx-lm-omni not installed. Install it to enable audio transcription."
            }
        
        try:
            import librosa
            import numpy as np
            import os
            
            # Check if file exists
            if not os.path.exists(audio_path):
                return {
                    "type": "error",
                    "command": "generate-transcript",
                    "error": f"Audio file not found: {audio_path}"
                }
            
            # Load audio file
            if audio_path.endswith('.wav'):
                audio, sr = librosa.load(audio_path, sr=16000, mono=True)
            elif audio_path.endswith('.pcm'):
                # Read raw PCM (s16le, 16kHz, mono)
                with open(audio_path, 'rb') as f:
                    pcm_data = np.frombuffer(f.read(), dtype=np.int16)
                audio = pcm_data.astype(np.float32) / 32768.0  # Convert to float32 [-1, 1]
            else:
                return {
                    "type": "error",
                    "command": "generate-transcript",
                    "error": f"Unsupported audio format: {audio_path}"
                }
            
            # Handle empty audio
            if len(audio) == 0:
                return {
                    "type": "response",
                    "command": "generate-transcript",
                    "language": "unknown",
                    "transcript": ""
                }
            
            # Clear ExtendedEmbedding queue to prevent state leakage between calls.
            # Qwen-Omni structure: model.thinker.model.embed_tokens
            embed = None
            if hasattr(self.model, 'thinker') and hasattr(self.model.thinker, 'model'):
                embed = getattr(self.model.thinker.model, 'embed_tokens', None)
            elif hasattr(self.model, 'language_model') and hasattr(self.model.language_model, 'model'):
                embed = getattr(self.model.language_model.model, 'embed_tokens', None)
            if embed and hasattr(embed, 'extended_embedding_queue'):
                embed.extended_embedding_queue.clear()
            
            # Generate transcript — structured JSON output with language detection
            prompt_text = 'Detect the language spoken. Transcribe word for word in the detected language. Do NOT translate. Respond in JSON: {"language": "...", "transcript": "..."}'

            # Build chat messages with audio and use apply_chat_template to encode.
            # apply_chat_template processes audio through the audio tower and injects
            # audio embedding tokens into the token sequence. The resulting token IDs
            # are then passed directly to generate() as the prompt.
            messages = [
                {"role": "user", "content": prompt_text, "audio": audio}
            ]
            token_ids = self.tokenizer.apply_chat_template(messages, add_generation_prompt=True)

            response = mlx_lm_generate(
                self.model,
                self.tokenizer,
                prompt=token_ids,
                max_tokens=2000,
                prefill_step_size=32768,
                verbose=False
            )

            # Parse structured JSON response from model
            import json as _json
            response_text = response.strip()
            # Strip markdown code fence if present
            if response_text.startswith("```"):
                lines = response_text.split("\n")
                # Remove first line (```json) and last line (```)
                lines = [l for l in lines if not l.strip().startswith("```")]
                response_text = "\n".join(lines).strip()

            try:
                parsed = _json.loads(response_text)
                language = parsed.get("language", "unknown")
                transcript = parsed.get("transcript", response_text)
            except _json.JSONDecodeError:
                # Fallback: use raw response as transcript
                transcript = response_text
                language = "unknown"
            
            return {
                "type": "response",
                "command": "generate-transcript",
                "language": language,
                "transcript": transcript.strip()
            }
            
        except ImportError as e:
            return {
                "type": "error",
                "command": "generate-transcript",
                "error": f"Missing audio dependencies: {str(e)}"
            }
        except Exception as e:
            return {
                "type": "error",
                "command": "generate-transcript",
                "error": f"Transcription failed: {str(e)}"
            }
    
    def copilot_analyze(self, audio_path: str, context: str) -> Dict[str, Any]:
        """Analyze audio chunk with running context for Co-Pilot.
        
        Args:
            audio_path: Path to audio file (.wav format)
            context: Running context (previous cycle's summary, empty for first cycle)
            
        Returns:
            Dict with type, command, and structured analysis fields
        """
        if self.model is None:
            return {
                "type": "error",
                "command": "copilot-analyze",
                "error": "No model loaded"
            }
        
        # Check capabilities from catalog
        if "audio" not in self.capabilities:
            return {
                "type": "error",
                "command": "copilot-analyze",
                "error": "Model does not support audio analysis"
            }
        
        # Check if mlx_lm_omni.generate is available
        if not OMNI_AVAILABLE or mlx_omni_generate is None:
            return {
                "type": "error",
                "command": "copilot-analyze",
                "error": "mlx-lm-omni not installed. Install it to enable Co-Pilot analysis."
            }
        
        try:
            import librosa
            import numpy as np
            import os
            import json as _json
            
            # Check if file exists
            if not os.path.exists(audio_path):
                return {
                    "type": "error",
                    "command": "copilot-analyze",
                    "error": f"Audio file not found: {audio_path}"
                }
            
            # Load audio file
            audio, sr = librosa.load(audio_path, sr=16000, mono=True)
            
            # Handle empty audio
            if len(audio) == 0:
                return {
                    "type": "response",
                    "command": "copilot-analyze",
                    "new_content": "",
                    "updated_summary": context,
                    "key_points": [],
                    "decisions": [],
                    "action_items": [],
                    "open_questions": [],
                    "suggested_questions": [],
                    "key_concepts": []
                }
            
            # Clear ExtendedEmbedding queue to prevent state leakage
            if hasattr(self.model, 'language_model') and hasattr(self.model.language_model, 'model'):
                embed = self.model.language_model.model.embed_tokens
                if hasattr(embed, 'extended_embedding_queue'):
                    embed.extended_embedding_queue.clear()
            
            # Construct prompt based on whether this is first cycle or subsequent
            if context:
                prompt_text = f"""Previous conversation summary:
{context}

Analyze the new audio segment and provide:
1. What new content was discussed
2. Updated summary of the entire conversation so far
3. Key points mentioned
4. Any decisions made
5. Action items identified
6. Open questions raised
7. Suggested questions to ask next (with reasons)
8. Key concepts (technical terms, names, topics) with brief context

Respond in JSON format with these exact fields:
{{"new_content": "...", "updated_summary": "...", "key_points": [...], "decisions": [...], "action_items": [...], "open_questions": [...], "suggested_questions": [{{"question": "...", "reason": "..."}}], "key_concepts": [{{"term": "...", "context": "..."}}]}}"""
            else:
                prompt_text = """This is the start of a conversation. Analyze the audio and provide:
1. What was discussed
2. Summary of the conversation
3. Key points mentioned
4. Any decisions made
5. Action items identified
6. Open questions raised
7. Suggested questions to ask next (with reasons)
8. Key concepts (technical terms, names, topics) with brief context

Respond in JSON format with these exact fields:
{"new_content": "...", "updated_summary": "...", "key_points": [...], "decisions": [...], "action_items": [...], "open_questions": [...], "suggested_questions": [{"question": "...", "reason": "..."}], "key_concepts": [{"term": "...", "context": "..."}]}"""
            
            # Build messages with audio
            messages = [
                {"role": "user", "content": prompt_text, "audio": audio}
            ]
            token_ids = self.tokenizer.apply_chat_template(messages, add_generation_prompt=True)
            
            # Generate response
            response = mlx_omni_generate(
                self.model,
                self.tokenizer,
                prompt=token_ids,
                max_tokens=2000,
                prefill_step_size=32768,
                verbose=False
            )
            
            # Parse JSON response
            response_text = response.strip()
            
            # Strip markdown code fence if present
            if response_text.startswith("```"):
                lines = response_text.split("\n")
                lines = [l for l in lines if not l.strip().startswith("```")]
                response_text = "\n".join(lines).strip()
            
            try:
                parsed = _json.loads(response_text)
                
                # Validate required fields, provide defaults for missing (graceful parsing)
                result = {
                    "type": "response",
                    "command": "copilot-analyze",
                    "new_content": parsed.get("new_content", ""),
                    "updated_summary": parsed.get("updated_summary", ""),
                    "key_points": parsed.get("key_points", []),
                    "decisions": parsed.get("decisions", []),
                    "action_items": parsed.get("action_items", []),
                    "open_questions": parsed.get("open_questions", []),
                    "suggested_questions": parsed.get("suggested_questions", []),
                    "key_concepts": parsed.get("key_concepts", [])
                }
                
                return result
                
            except _json.JSONDecodeError:
                # Partial JSON - return what we can parse with defaults
                return {
                    "type": "response",
                    "command": "copilot-analyze",
                    "new_content": response_text,
                    "updated_summary": response_text,
                    "key_points": [],
                    "decisions": [],
                    "action_items": [],
                    "open_questions": [],
                    "suggested_questions": [],
                    "key_concepts": []
                }
        
        except ImportError as e:
            return {
                "type": "error",
                "command": "copilot-analyze",
                "error": f"Missing audio dependencies: {str(e)}"
            }
        except Exception as e:
            return {
                "type": "error",
                "command": "copilot-analyze",
                "error": f"Co-Pilot analysis failed: {str(e)}"
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
            capabilities = command_data.get("capabilities")  # Get capabilities from Rust
            if not model_path:
                return {"type": "error", "command": command, "error": "Missing model_path"}
            return self.load_model(model_path, capabilities)
        
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
        
        elif command == "chat":
            messages = command_data.get("messages")
            if not messages:
                return {"type": "error", "command": command, "error": "Missing messages"}
            return self.chat(messages)
        
        elif command == "generate-transcript":
            audio_path = command_data.get("audio_path")
            if not audio_path:
                return {"type": "error", "command": command, "error": "Missing audio_path"}
            return self.generate_transcript(audio_path)
        
        elif command == "copilot-analyze":
            audio_path = command_data.get("audio_path")
            context = command_data.get("context", "")
            if not audio_path:
                return {"type": "error", "command": command, "error": "Missing audio_path"}
            return self.copilot_analyze(audio_path, context)
        
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
    # Apply runtime patches for mlx-lm-omni before starting server
    apply_runtime_patches()
    
    server = MLXServer()
    server.run()
