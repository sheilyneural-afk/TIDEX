#!/usr/bin/env python3
"""Embedded persistent Hugging Face execution worker for TIDE-X.

Loaded by Rust from compile-time embedded source. It performs real model
generation, hidden-state measurement, activation intervention, and bound local
sparse-autoencoder analysis. Hello reports a domain-separated runtime identity
digest, not a catalog of package versions.
"""

from __future__ import annotations

import argparse
import hashlib
import importlib.metadata
import json
import math
import os
import re
import struct
import sys
import time
from pathlib import Path
from typing import Any

HELLO_SCHEMA = "cerebro.tidex.hf_worker_hello/v1"
RESPONSE_SCHEMA = "cerebro.cross_model.hf_worker_response/v1"
RUNTIME_IDENTITY_DOMAIN = b"CEREBRO:TIDEX:HF-RUNTIME-IDENTITY:v1\0"
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


def digest_domain(domain: bytes, payload: bytes) -> str:
    hasher = hashlib.sha256()
    hasher.update(domain)
    hasher.update(payload)
    return hasher.hexdigest()


def pep503_name(name: str) -> str:
    return re.sub(r"[-_.]+", "-", name).lower()


def installed_runtime_identity() -> str:
    packages: dict[str, str] = {}
    for distribution in importlib.metadata.distributions():
        raw_name = distribution.metadata["Name"]
        require(isinstance(raw_name, str) and raw_name.strip(), "hf_runtime_distribution_unnamed")
        key = pep503_name(raw_name)
        version = distribution.version
        require(version.strip(), f"hf_runtime_distribution_unversioned:{raw_name}")
        if key in packages:
            require(packages[key] == version, f"hf_runtime_distribution_conflict:{key}")
            continue
        packages[key] = version
    payload = {
        "implementation": sys.implementation.name,
        "packages": {key: packages[key] for key in sorted(packages)},
        "python_minor": f"{sys.version_info.major}.{sys.version_info.minor}",
    }
    return digest_domain(
        RUNTIME_IDENTITY_DOMAIN,
        json.dumps(payload, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode(),
    )


def run_hf_worker(args: argparse.Namespace) -> int:
    require(sys.version_info[:2] == (3, 12), f"hf_worker_python_unsupported:{sys.version}")
    require(os.environ.get("PYTHONNOUSERSITE") == "1", "hf_worker_usersite_forbidden")
    require(os.environ.get("HF_HUB_OFFLINE") == "1", "hf_worker_hub_offline_required")
    require(os.environ.get("TOKENIZERS_PARALLELISM") == "false", "hf_worker_tokenizers_parallelism_required")
    require(os.environ.get("TRANSFORMERS_NO_TF") == "1", "hf_worker_transformers_tf_forbidden")
    require(os.environ.get("USE_TF") == "0", "hf_worker_tf_forbidden")

    import nnsight
    import torch
    from safetensors.torch import load_file
    from transformers import AutoModelForCausalLM, AutoTokenizer

    nnsight_version = importlib.metadata.version("nnsight")
    require(nnsight.__version__ == nnsight_version, "hf_nnsight_version_identity_mismatch")
    runtime_identity_sha256 = installed_runtime_identity()

    snapshot = Path(args.model_dir)
    checkpoint_link = Path(args.checkpoint)
    config_link = Path(args.config)
    tokenizer_link = Path(args.tokenizer)
    checkpoint = exact_snapshot_file(snapshot, checkpoint_link, "model.safetensors")
    config_path = exact_snapshot_file(snapshot, config_link, "config.json")
    tokenizer_path = exact_snapshot_file(snapshot, tokenizer_link, "tokenizer.json")

    require(1 <= args.threads <= 256, "hf_worker_thread_count_invalid")
    torch.set_num_interop_threads(args.threads)
    torch.set_num_threads(args.threads)

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
        dtype=torch.float32,
        low_cpu_mem_usage=False,
    )
    model.eval()
    load_duration_ns = time.monotonic_ns() - started

    require(sha256_file(checkpoint) == checkpoint_sha_before, "hf_worker_checkpoint_changed_during_load")
    require(sha256_file(config_path) == config_sha_before, "hf_worker_config_changed_during_load")
    require(sha256_file(tokenizer_path) == tokenizer_sha_before, "hf_worker_tokenizer_changed_during_load")

    cfg = model.config
    hidden_size = int(cfg.hidden_size)
    intermediate_size = int(cfg.intermediate_size)
    num_layers = int(cfg.num_hidden_layers)
    num_heads = int(cfg.num_attention_heads)
    vocab_size = int(cfg.vocab_size)
    max_positions = int(cfg.max_position_embeddings)
    architectures = [str(item) for item in cfg.architectures]
    require(
        all(v > 0 for v in [hidden_size, intermediate_size, num_layers, num_heads, vocab_size, max_positions]),
        "hf_worker_model_geometry_missing",
    )
    require(bool(architectures) and all(item.strip() for item in architectures), "hf_worker_architecture_list_invalid")

    decoder = getattr(model, "model", None)
    decoder_layers = getattr(decoder, "layers", None)
    require(decoder_layers is not None and len(decoder_layers) == num_layers, "hf_worker_decoder_layers_unavailable")
    parameter_count = sum(int(parameter.numel()) for parameter in model.parameters())
    require(parameter_count > 0, "hf_worker_parameter_count_invalid")

    hello = {
        "schema": HELLO_SCHEMA,
        "model_type": str(cfg.model_type),
        "architectures": architectures,
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
        "runtime_identity_sha256": runtime_identity_sha256,
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
        require(nnsight_version == importlib.metadata.version("nnsight"), "hf_nnsight_version_identity_mismatch")
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
                    == {
                        "prompt",
                        "module_path",
                        "token_from_end",
                        "sae_dir",
                        "weights_path",
                        "config_path",
                        "top_k",
                    },
                    "hf_sae_payload_fields_invalid",
                )
                top_k = int(payload["top_k"])
                require(1 <= top_k <= 4096, "hf_sae_top_k_invalid")
                sae_dir = Path(payload["sae_dir"])
                weights = exact_snapshot_file(sae_dir, Path(payload["weights_path"]), "sae.safetensors")
                config_file = exact_snapshot_file(sae_dir, Path(payload["config_path"]), "config.json")
                values, duration = capture_nnsight_vector(
                    payload["prompt"], payload["module_path"], int(payload["token_from_end"])
                )
                started_sae = time.monotonic_ns()
                cfg = json.loads(config_file.read_text(encoding="utf-8"))
                require(isinstance(cfg, dict) and set(cfg) == {"activation", "d_in", "d_sae"}, "hf_sae_config_fields_invalid")
                require(cfg["activation"] == "relu", "hf_sae_activation_unsupported")
                d_in = int(cfg["d_in"])
                d_sae = int(cfg["d_sae"])
                require(d_in == len(values) and d_sae > 0, "hf_sae_geometry_mismatch")
                tensors = load_file(str(weights))
                require(set(tensors) == {"W_dec", "W_enc", "b_dec", "b_enc"}, "hf_sae_tensor_set_invalid")
                w_enc = tensors["W_enc"].detach().to(dtype=torch.float64, device="cpu")
                b_enc = tensors["b_enc"].detach().to(dtype=torch.float64, device="cpu")
                w_dec = tensors["W_dec"].detach().to(dtype=torch.float64, device="cpu")
                b_dec = tensors["b_dec"].detach().to(dtype=torch.float64, device="cpu")
                require(tuple(w_enc.shape) == (d_in, d_sae), "hf_sae_w_enc_shape_invalid")
                require(tuple(b_enc.shape) == (d_sae,), "hf_sae_b_enc_shape_invalid")
                require(tuple(w_dec.shape) == (d_sae, d_in), "hf_sae_w_dec_shape_invalid")
                require(tuple(b_dec.shape) == (d_in,), "hf_sae_b_dec_shape_invalid")
                source = torch.tensor(values, dtype=torch.float64)
                with torch.no_grad():
                    features = torch.relu(source @ w_enc + b_enc)
                    reconstruction = features @ w_dec + b_dec
                require(features.numel() == d_sae, "hf_sae_features_invalid")
                require(reconstruction.numel() == d_in, "hf_sae_reconstruction_shape_mismatch")
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
                    "sae_weights_sha256": sha256_file(weights),
                    "sae_config_sha256": sha256_file(config_file),
                    "d_in": d_in,
                    "d_sae": d_sae,
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
