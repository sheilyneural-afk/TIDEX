#!/usr/bin/env python3
"""Embedded persistent Hugging Face execution worker for CEREBRO3.

Loaded by Rust from compile-time embedded source. It performs only real model
generation, hidden-state measurement, activation intervention and clearing.
"""

from __future__ import annotations

import argparse
import hashlib
import importlib.metadata
import json
import math
import os
import struct
import sys
import time
from pathlib import Path
from typing import Any

HELLO_SCHEMA = "cerebro.cross_model.hf_worker_hello/v1"
RESPONSE_SCHEMA = "cerebro.cross_model.hf_worker_response/v1"
MAX_LINE_BYTES = 32 * 1024 * 1024
MAX_PROMPT_CHARS = 1_048_576
MAX_GENERATED_TOKENS = 32_768
MAX_STEERING_VALUES = 65_536


def sha256_file(path: Path) -> str:
    hasher = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(4 * 1024 * 1024), b""):
            hasher.update(chunk)
    return hasher.hexdigest()


def canonical_sha256(value: Any) -> str:
    raw = json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
    return hashlib.sha256(raw).hexdigest()


def f64_vector_sha256(values: list[float]) -> str:
    hasher = hashlib.sha256()
    hasher.update(b"CEREBRO:CROSS-MODEL:F64-VECTOR:v1\0")
    hasher.update(struct.pack("<Q", len(values)))
    for value in values:
        hasher.update(struct.pack("<d", value))
    return hasher.hexdigest()


def require(condition: bool, message: str) -> None:
    if not condition:
        raise RuntimeError(message)


def exact_snapshot_file(snapshot: Path, explicit: Path, name: str) -> Path:
    require(snapshot.is_absolute() and explicit.is_absolute(), "hf_worker_paths_must_be_absolute")
    require(snapshot.is_dir(), "hf_worker_snapshot_missing")
    candidate = snapshot / name
    require(candidate.exists() and explicit.exists(), f"hf_worker_required_file_missing:{name}")
    require(candidate.samefile(explicit), f"hf_worker_file_binding_mismatch:{name}")
    resolved = explicit.resolve(strict=True)
    require(resolved.is_file(), f"hf_worker_file_not_regular:{name}")
    return resolved


def response(request_id: str, operation: str, payload: dict[str, Any]) -> dict[str, Any]:
    value = {
        "schema": RESPONSE_SCHEMA,
        "request_id": request_id,
        "operation": operation,
        "ok": True,
        "payload": payload,
    }
    value["response_sha256"] = canonical_sha256(value)
    return value


def failure(request_id: str, operation: str, error: BaseException) -> dict[str, Any]:
    value = {
        "schema": RESPONSE_SCHEMA,
        "request_id": request_id,
        "operation": operation,
        "ok": False,
        "error": f"{type(error).__name__}:{error}",
    }
    value["response_sha256"] = canonical_sha256(value)
    return value


def run_hf_worker(args: argparse.Namespace) -> int:
    os.environ.setdefault("TOKENIZERS_PARALLELISM", "false")
    os.environ.setdefault("TRANSFORMERS_NO_TF", "1")
    os.environ.setdefault("USE_TF", "0")
    os.environ.setdefault("HF_HUB_OFFLINE", "1")
    os.environ.setdefault("PYTHONNOUSERSITE", "1")

    import torch
    from transformers import AutoModelForCausalLM, AutoTokenizer

    def importable_package_version(name: str) -> str | None:
        candidates = [name.lower().replace("-", "_")]
        if name.lower() == "sae-lens":
            candidates = ["sae_lens", "sae-lens"]
        if name.lower() == "nnsight":
            candidates = ["nnsight"]
        for candidate in candidates:
            try:
                __import__(candidate)
                return importlib.metadata.version(name)
            except Exception:
                continue
        return None

    nnsight_version = importable_package_version("nnsight")
    sae_lens_version = importable_package_version("sae-lens")

    snapshot = Path(args.model_dir)
    checkpoint_link = Path(args.checkpoint)
    config_link = Path(args.config)
    tokenizer_link = Path(args.tokenizer)
    checkpoint = exact_snapshot_file(snapshot, checkpoint_link, "model.safetensors")
    config_path = exact_snapshot_file(snapshot, config_link, "config.json")
    tokenizer_path = exact_snapshot_file(snapshot, tokenizer_link, "tokenizer.json")

    require(1 <= args.threads <= 256, "hf_worker_thread_count_invalid")
    torch.set_num_threads(args.threads)
    if hasattr(torch, "set_num_interop_threads"):
        try:
            torch.set_num_interop_threads(max(1, min(args.threads, 8)))
        except RuntimeError:
            pass

    checkpoint_sha_before = sha256_file(checkpoint)
    config_sha_before = sha256_file(config_path)
    tokenizer_sha_before = sha256_file(tokenizer_path)

    started = time.monotonic_ns()
    tokenizer = AutoTokenizer.from_pretrained(
        str(snapshot), local_files_only=True, trust_remote_code=False
    )
    model = AutoModelForCausalLM.from_pretrained(
        str(snapshot),
        local_files_only=True,
        trust_remote_code=False,
        torch_dtype=torch.float32,
        low_cpu_mem_usage=False,
    )
    model.eval()
    load_duration_ns = time.monotonic_ns() - started

    require(sha256_file(checkpoint) == checkpoint_sha_before, "hf_worker_checkpoint_changed_during_load")
    require(sha256_file(config_path) == config_sha_before, "hf_worker_config_changed_during_load")
    require(sha256_file(tokenizer_path) == tokenizer_sha_before, "hf_worker_tokenizer_changed_during_load")

    cfg = model.config
    hidden_size = int(getattr(cfg, "hidden_size", 0) or 0)
    intermediate_size = int(getattr(cfg, "intermediate_size", 0) or 0)
    num_layers = int(getattr(cfg, "num_hidden_layers", 0) or 0)
    num_heads = int(getattr(cfg, "num_attention_heads", 0) or 0)
    vocab_size = int(getattr(cfg, "vocab_size", 0) or 0)
    max_positions = int(getattr(cfg, "max_position_embeddings", 0) or 0)
    require(
        all(v > 0 for v in [hidden_size, intermediate_size, num_layers, num_heads, vocab_size, max_positions]),
        "hf_worker_model_geometry_missing",
    )

    decoder = getattr(model, "model", None)
    decoder_layers = getattr(decoder, "layers", None)
    require(decoder_layers is not None and len(decoder_layers) == num_layers, "hf_worker_decoder_layers_unavailable")
    parameter_count = sum(int(parameter.numel()) for parameter in model.parameters())
    require(parameter_count > 0, "hf_worker_parameter_count_invalid")

    hello = {
        "schema": HELLO_SCHEMA,
        "model_type": str(getattr(cfg, "model_type", "")),
        "architectures": list(getattr(cfg, "architectures", None) or []),
        "hidden_size": hidden_size,
        "intermediate_size": intermediate_size,
        "num_hidden_layers": num_layers,
        "num_attention_heads": num_heads,
        "vocab_size": vocab_size,
        "max_position_embeddings": max_positions,
        "parameter_count": parameter_count,
        "checkpoint_sha256": checkpoint_sha_before,
        "config_sha256": config_sha_before,
        "tokenizer_sha256": tokenizer_sha_before,
        "load_duration_ns": load_duration_ns,
        "activation_semantics": "decoder_layer_output_last_prompt_token/v1",
        "steering_semantics": "decoder_layer_output_additive_broadcast/v1",
        "nnsight_version": nnsight_version,
        "sae_lens_version": sae_lens_version,
    }
    hello["manifest_sha256"] = canonical_sha256(hello)
    print(json.dumps(hello, separators=(",", ":")), flush=True)

    active_steering: dict[int, tuple[torch.Tensor, float, str]] = {}

    def tokenize(prompt: str) -> dict[str, torch.Tensor]:
        require(isinstance(prompt, str) and 0 < len(prompt) <= MAX_PROMPT_CHARS, "hf_worker_prompt_invalid")
        encoded = tokenizer(prompt, return_tensors="pt", add_special_tokens=True)
        require("input_ids" in encoded and encoded["input_ids"].numel() > 0, "hf_worker_tokenization_empty")
        require(encoded["input_ids"].shape[-1] <= max_positions, "hf_worker_prompt_context_exceeded")
        return encoded

    def steering_hook(layer_index: int):
        def hook(_module, _inputs, output):
            vector, strength, _digest = active_steering[layer_index]
            if isinstance(output, tuple):
                require(len(output) > 0 and torch.is_tensor(output[0]), "hf_worker_layer_output_invalid")
                hidden = output[0]
                require(hidden.shape[-1] == hidden_size, "hf_worker_layer_hidden_size_mismatch")
                delta = vector.to(device=hidden.device, dtype=hidden.dtype).view(1, 1, -1)
                return (hidden + strength * delta, *output[1:])
            require(torch.is_tensor(output), "hf_worker_layer_output_invalid")
            require(output.shape[-1] == hidden_size, "hf_worker_layer_hidden_size_mismatch")
            delta = vector.to(device=output.device, dtype=output.dtype).view(1, 1, -1)
            return output + strength * delta
        return hook

    def with_steering(callable_):
        handles = []
        try:
            for layer_index in sorted(active_steering):
                handles.append(decoder_layers[layer_index].register_forward_hook(steering_hook(layer_index)))
            return callable_()
        finally:
            for handle in handles:
                handle.remove()

    def validate_module_path(module_path: str) -> list[str]:
        require(isinstance(module_path, str) and 0 < len(module_path) <= 4096, "hf_module_path_invalid")
        parts = module_path.split(".")
        require(
            all(
                part
                and not part.startswith("_")
                and all(character.isalnum() or character == "_" for character in part)
                for part in parts
            ),
            "hf_module_path_invalid",
        )
        return parts

    def resolve_envoy(root: Any, module_path: str) -> Any:
        current = root
        for part in validate_module_path(module_path):
            if part.isdigit():
                current = current[int(part)]
            else:
                current = getattr(current, part)
        return current

    def capture_nnsight_vector(prompt: str, module_path: str, token_from_end: int) -> tuple[list[float], int]:
        require(nnsight_version is not None, "hf_nnsight_not_installed")
        require(isinstance(token_from_end, int) and 0 <= token_from_end <= 65_535, "hf_nnsight_token_index_invalid")
        import nnsight

        encoded = tokenize(prompt)
        wrapped = nnsight.NNsight(model)
        target = resolve_envoy(wrapped, module_path)
        started_request = time.monotonic_ns()
        with wrapped.trace(**encoded):
            captured = target.output.save()
        if isinstance(captured, tuple):
            require(len(captured) > 0, "hf_nnsight_module_output_empty")
            captured = captured[0]
        require(torch.is_tensor(captured), "hf_nnsight_module_output_not_tensor")
        captured = captured.detach().to(dtype=torch.float64, device="cpu")
        if captured.ndim >= 3:
            require(captured.shape[0] == 1 and token_from_end < captured.shape[-2], "hf_nnsight_token_index_out_of_range")
            selected = captured[0, captured.shape[-2] - 1 - token_from_end]
        elif captured.ndim == 2:
            require(captured.shape[0] == 1 and token_from_end == 0, "hf_nnsight_nonsequence_token_index")
            selected = captured[0]
        elif captured.ndim == 1:
            require(token_from_end == 0, "hf_nnsight_nonsequence_token_index")
            selected = captured
        else:
            raise RuntimeError("hf_nnsight_module_output_scalar")
        selected = selected.contiguous().reshape(-1)
        require(0 < selected.numel() <= 1_048_576, "hf_nnsight_vector_size_invalid")
        values = [float(value) for value in selected.tolist()]
        require(all(math.isfinite(value) for value in values), "hf_nnsight_vector_nonfinite")
        return values, time.monotonic_ns() - started_request

    for raw_line in sys.stdin.buffer:
        if len(raw_line) > MAX_LINE_BYTES:
            print(json.dumps(failure("unknown", "unknown", RuntimeError("hf_worker_request_too_large")), separators=(",", ":")), flush=True)
            continue
        request_id = "unknown"
        operation = "unknown"
        try:
            request = json.loads(raw_line)
            require(isinstance(request, dict), "hf_worker_request_not_object")
            allowed_common = {"schema", "request_id", "operation", "payload"}
            require(set(request) == allowed_common, "hf_worker_request_fields_invalid")
            require(request.get("schema") == "cerebro.cross_model.hf_worker_request/v1", "hf_worker_request_schema_invalid")
            request_id = request.get("request_id")
            operation = request.get("operation")
            payload = request.get("payload")
            require(isinstance(request_id, str) and request_id, "hf_worker_request_id_invalid")
            require(isinstance(operation, str) and operation, "hf_worker_operation_invalid")
            require(isinstance(payload, dict), "hf_worker_payload_invalid")

            if operation == "generate":
                allowed = {"prompt", "max_new_tokens", "seed", "temperature", "top_p"}
                require(set(payload) == allowed, "hf_generate_payload_fields_invalid")
                prompt = payload["prompt"]
                max_new_tokens = int(payload["max_new_tokens"])
                seed = int(payload["seed"])
                temperature = float(payload["temperature"])
                top_p = float(payload["top_p"])
                require(1 <= max_new_tokens <= MAX_GENERATED_TOKENS, "hf_generate_token_limit_invalid")
                require(seed >= 0, "hf_generate_seed_invalid")
                require(math.isfinite(temperature) and temperature >= 0.0, "hf_generate_temperature_invalid")
                require(math.isfinite(top_p) and 0.0 < top_p <= 1.0, "hf_generate_top_p_invalid")
                encoded = tokenize(prompt)
                prompt_tokens = int(encoded["input_ids"].shape[-1])
                require(prompt_tokens + max_new_tokens <= max_positions, "hf_generate_context_exceeded")
                torch.manual_seed(seed)
                started_request = time.monotonic_ns()

                def generate_call():
                    kwargs = {
                        "max_new_tokens": max_new_tokens,
                        "use_cache": True,
                        "pad_token_id": tokenizer.eos_token_id,
                    }
                    if temperature > 0.0:
                        kwargs.update({"do_sample": True, "temperature": temperature, "top_p": top_p})
                    else:
                        kwargs.update({"do_sample": False})
                    with torch.no_grad():
                        return model.generate(**encoded, **kwargs)

                generated = with_steering(generate_call)
                generated_ids = generated[0, prompt_tokens:].tolist()
                text = tokenizer.decode(generated_ids, skip_special_tokens=True)
                eos_ids = tokenizer.eos_token_id
                if isinstance(eos_ids, int):
                    eos_ids = {eos_ids}
                elif isinstance(eos_ids, (list, tuple, set)):
                    eos_ids = set(eos_ids)
                else:
                    eos_ids = set()
                done_reason = "eos" if generated_ids and generated_ids[-1] in eos_ids else "length"
                payload_out = {
                    "text": text,
                    "prompt_eval_count": prompt_tokens,
                    "eval_count": len(generated_ids),
                    "total_duration_ns": time.monotonic_ns() - started_request,
                    "done_reason": done_reason,
                    "active_steering": [
                        {"layer_index": layer, "steering_sha256": active_steering[layer][2], "strength": active_steering[layer][1]}
                        for layer in sorted(active_steering)
                    ],
                }
                print(json.dumps(response(request_id, operation, payload_out), separators=(",", ":")), flush=True)

            elif operation == "activation":
                require(set(payload) == {"prompt", "layer_index"}, "hf_activation_payload_fields_invalid")
                prompt = payload["prompt"]
                layer_index = int(payload["layer_index"])
                require(0 <= layer_index < num_layers, "hf_activation_layer_invalid")
                encoded = tokenize(prompt)
                started_request = time.monotonic_ns()

                def activation_call():
                    with torch.no_grad():
                        return model(
                            **encoded,
                            output_hidden_states=True,
                            use_cache=False,
                            return_dict=True,
                        )

                output = with_steering(activation_call)
                hidden_states = output.hidden_states
                require(hidden_states is not None and len(hidden_states) == num_layers + 1, "hf_hidden_state_count_invalid")
                vector_tensor = hidden_states[layer_index + 1][0, -1].detach().to(dtype=torch.float64, device="cpu").contiguous()
                require(vector_tensor.numel() == hidden_size, "hf_hidden_state_dimension_invalid")
                values = [float(value) for value in vector_tensor.tolist()]
                require(all(math.isfinite(value) for value in values), "hf_hidden_state_nonfinite")
                payload_out = {
                    "layer_index": layer_index,
                    "values": values,
                    "vector_sha256": f64_vector_sha256(values),
                    "semantics": "decoder_layer_output_last_prompt_token/v1",
                    "total_duration_ns": time.monotonic_ns() - started_request,
                }
                print(json.dumps(response(request_id, operation, payload_out), separators=(",", ":")), flush=True)

            elif operation == "deep_instrumentation":
                require(
                    set(payload) == {"prompt", "module_path", "token_from_end"},
                    "hf_deep_instrumentation_payload_fields_invalid",
                )
                values, duration = capture_nnsight_vector(
                    payload["prompt"], payload["module_path"], int(payload["token_from_end"])
                )
                payload_out = {
                    "module_path": payload["module_path"],
                    "token_from_end": int(payload["token_from_end"]),
                    "values": values,
                    "values_sha256": f64_vector_sha256(values),
                    "backend": "nnsight",
                    "backend_version": nnsight_version,
                    "total_duration_ns": duration,
                }
                print(json.dumps(response(request_id, operation, payload_out), separators=(",", ":")), flush=True)

            elif operation == "sae_analysis":
                require(
                    set(payload)
                    == {"prompt", "module_path", "token_from_end", "release", "sae_id", "top_k"},
                    "hf_sae_payload_fields_invalid",
                )
                require(sae_lens_version is not None, "hf_sae_lens_not_installed")
                release = payload["release"]
                sae_id = payload["sae_id"]
                top_k = int(payload["top_k"])
                require(isinstance(release, str) and 0 < len(release) <= 4096, "hf_sae_release_invalid")
                require(isinstance(sae_id, str) and 0 < len(sae_id) <= 4096, "hf_sae_id_invalid")
                require(1 <= top_k <= 4096, "hf_sae_top_k_invalid")
                values, duration = capture_nnsight_vector(
                    payload["prompt"], payload["module_path"], int(payload["token_from_end"])
                )
                from sae_lens import SAE

                started_sae = time.monotonic_ns()
                sae = SAE.from_pretrained(release=release, sae_id=sae_id, device="cpu", dtype="float32")
                activation = torch.tensor(values, dtype=torch.float32)
                with torch.no_grad():
                    features = sae.encode(activation)
                    reconstruction = sae.decode(features)
                require(torch.is_tensor(features) and features.numel() > 0, "hf_sae_features_invalid")
                features = features.detach().to(dtype=torch.float64, device="cpu").reshape(-1)
                reconstruction = reconstruction.detach().to(dtype=torch.float64, device="cpu").reshape(-1)
                require(reconstruction.numel() == len(values), "hf_sae_reconstruction_shape_mismatch")
                source = torch.tensor(values, dtype=torch.float64)
                denominator = float(torch.linalg.vector_norm(source))
                require(math.isfinite(denominator) and denominator > 0.0, "hf_sae_input_degenerate")
                reconstruction_error = float(torch.linalg.vector_norm(reconstruction - source) / denominator)
                require(math.isfinite(reconstruction_error), "hf_sae_reconstruction_nonfinite")
                feature_count = int(features.numel())
                active_feature_count = int(torch.count_nonzero(features.abs() > 1.0e-12).item())
                take = min(top_k, feature_count)
                _, indices = torch.topk(features.abs(), k=take)
                top_features = [
                    {"index": int(index), "activation": float(features[int(index)])}
                    for index in indices.tolist()
                ]
                payload_out = {
                    "module_path": payload["module_path"],
                    "token_from_end": int(payload["token_from_end"]),
                    "release": release,
                    "sae_id": sae_id,
                    "sae_lens_version": sae_lens_version,
                    "input_sha256": f64_vector_sha256(values),
                    "feature_count": feature_count,
                    "active_feature_count": active_feature_count,
                    "top_features": top_features,
                    "reconstruction_relative_error": reconstruction_error,
                    "total_duration_ns": duration + (time.monotonic_ns() - started_sae),
                }
                print(json.dumps(response(request_id, operation, payload_out), separators=(",", ":")), flush=True)

            elif operation == "set_steering":
                require(set(payload) == {"layer_index", "values", "strength"}, "hf_steering_payload_fields_invalid")
                layer_index = int(payload["layer_index"])
                values = payload["values"]
                strength = float(payload["strength"])
                require(0 <= layer_index < num_layers, "hf_steering_layer_invalid")
                require(isinstance(values, list) and len(values) == hidden_size and len(values) <= MAX_STEERING_VALUES, "hf_steering_dimension_invalid")
                values = [float(value) for value in values]
                require(all(math.isfinite(value) for value in values), "hf_steering_nonfinite")
                require(math.isfinite(strength) and strength > 0.0, "hf_steering_strength_invalid")
                digest = f64_vector_sha256(values)
                active_steering[layer_index] = (torch.tensor(values, dtype=torch.float64), strength, digest)
                payload_out = {
                    "layer_index": layer_index,
                    "steering_sha256": digest,
                    "strength": strength,
                    "active_steering_count": len(active_steering),
                }
                print(json.dumps(response(request_id, operation, payload_out), separators=(",", ":")), flush=True)

            elif operation == "clear_steering":
                require(not payload, "hf_clear_steering_payload_must_be_empty")
                active_steering.clear()
                print(json.dumps(response(request_id, operation, {"active_steering_count": 0}), separators=(",", ":")), flush=True)

            elif operation == "shutdown":
                require(not payload, "hf_shutdown_payload_must_be_empty")
                print(json.dumps(response(request_id, operation, {"shutdown": True}), separators=(",", ":")), flush=True)
                return 0

            else:
                raise RuntimeError("hf_worker_operation_unknown")
        except BaseException as error:
            print(json.dumps(failure(str(request_id), str(operation), error), separators=(",", ":")), flush=True)

    return 0



def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--model-dir", required=True)
    parser.add_argument("--checkpoint", required=True)
    parser.add_argument("--config", required=True)
    parser.add_argument("--tokenizer", required=True)
    parser.add_argument("--threads", type=int, required=True)
    return parser.parse_args()


def main() -> int:
    return run_hf_worker(parse_args())


if __name__ == "__main__":
    raise SystemExit(main())
