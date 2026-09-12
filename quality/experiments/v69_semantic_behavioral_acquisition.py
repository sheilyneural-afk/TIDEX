#!/usr/bin/env python3
"""V69: real semantic PRE/POST behavioral acquisition on SmolLM2 (zero receptor opt).

Capacity C = held-out Python discrete semantics (set cardinality + modulo) that the
virgin Instruct receptor fails under blind chat generation. Donor behavior is the
ground-truth Python evaluation of the same expressions (measured, not fabricated).

Delta path: V68-style measured response sensitivity on final LlamaRMSNorm → Rust
`tidex receiver compile` → `materialize` standalone checkpoint. No LoRA, no PEFT,
no backprop, optimizer_steps=0 on the receptor.

Claim boundary is fail-closed: new_semantic_capability_transfer_established is true
only if PRE generation fails C, POST generation passes C under the same scorer,
and conservation controls do not collapse. Margin-tracking alone never flips the
semantic flag.
"""
from __future__ import annotations

import argparse
import fcntl
import gc
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Any

os.environ.setdefault("HF_HUB_OFFLINE", "1")
os.environ.setdefault("TOKENIZERS_PARALLELISM", "false")
os.environ.setdefault("USE_TF", "0")
os.environ.setdefault("USE_FLAX", "0")

import torch
from transformers import AutoModelForCausalLM, AutoTokenizer

SOURCE = Path(__file__).resolve()
REPO = SOURCE.parents[2]
BASE_SHA = "f55217be716b6a997b97b9d8d7eb6fad02e00858f5010ec24f64603c3a98a0e8"
SCHEMA = "tidex.v69_semantic_behavioral_acquisition/v1"
MODEL_FILES = (
    "config.json",
    "generation_config.json",
    "tokenizer.json",
    "tokenizer_config.json",
    "special_tokens_map.json",
    "merges.txt",
    "vocab.json",
)


def canonical(value: Any) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":"), allow_nan=False) + "\n").encode()


def sha_file(path: Path) -> str:
    result = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(4 * 1024 * 1024), b""):
            result.update(chunk)
    return result.hexdigest()


def write_new(path: Path, data: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    if path.exists():
        if path.is_symlink() or path.read_bytes() != data:
            raise RuntimeError(f"immutable output collision: {path}")
        return
    with path.open("xb") as stream:
        stream.write(data)
        stream.flush()
        os.fsync(stream.fileno())
    path.chmod(0o600)


def copy_new(source: Path, destination: Path) -> None:
    if source.is_symlink() or not source.is_file():
        raise RuntimeError(f"reproducibility source is not a regular file: {source}")
    destination.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    if destination.exists():
        raise RuntimeError(f"reproducibility destination already exists: {destination}")
    with source.open("rb") as reader, destination.open("xb") as writer:
        shutil.copyfileobj(reader, writer, length=4 * 1024 * 1024)
        writer.flush()
        os.fsync(writer.fileno())
    destination.chmod(0o600)
    if sha_file(source) != sha_file(destination):
        raise RuntimeError("reproducibility copy digest mismatch")


def reproducibility_source_files() -> list[Path]:
    files = [REPO / name for name in (
        "Cargo.toml", "Cargo.lock", "build.rs", "rust-toolchain.toml",
        "deny.toml", ".cargo/config.toml",
    )]
    for relative in ("src", "tests", "quality/experiments"):
        for path in (REPO / relative).rglob("*"):
            if path.is_file() and not path.is_symlink() and "__pycache__" not in path.parts:
                files.append(path)
    unique = sorted(set(files))
    if any(not path.is_relative_to(REPO) for path in unique):
        raise RuntimeError("reproducibility source escaped repository")
    return unique


def freeze_reproducibility_bundle(destination: Path, binary: Path) -> dict[str, Any]:
    if destination.exists():
        raise RuntimeError("reproducibility bundle already exists")
    destination.mkdir(parents=True, mode=0o700)
    source_root = destination / "source"
    source_hashes: dict[str, str] = {}
    for source in reproducibility_source_files():
        relative = source.relative_to(REPO)
        target = source_root / relative
        copy_new(source, target)
        source_hashes[str(relative)] = sha_file(target)
    binary_target = destination / "bin" / "tidex"
    copy_new(binary, binary_target)
    binary_target.chmod(0o700)
    if SOURCE not in reproducibility_source_files():
        raise RuntimeError("collector is outside the frozen source set")
    head = subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=REPO, capture_output=True, text=True, check=True
    ).stdout.strip()
    manifest = {
        "schema": "tidex.experiment_reproducibility_bundle/v1",
        "git_head": head,
        "source_file_count": len(source_hashes),
        "source_sha256": source_hashes,
        "tidex_binary_sha256": sha_file(binary_target),
        "collector_sha256": source_hashes[str(SOURCE.relative_to(REPO))],
        "rebuild_scope": "complete canonical Cargo build inputs plus src, tests, and quality/experiments trees",
    }
    write_new(destination / "manifest.json", canonical(manifest))
    return {"path": str(destination / "manifest.json"), "sha256": sha_file(destination / "manifest.json")}


def put(root: Path, value: Any) -> dict[str, str]:
    data = canonical(value)
    sha = hashlib.sha256(data).hexdigest()
    path = root / "state/probe_inputs/by-sha" / f"{sha}.json"
    write_new(path, data)
    return {"path": str(path), "sha256": sha}


def read_reference(reference: dict[str, str]) -> Any:
    path = Path(reference["path"])
    if path.is_symlink() or sha_file(path) != reference["sha256"]:
        raise RuntimeError("input reference changed")
    return json.loads(path.read_bytes())


def tidex(binary: Path, root: Path, *arguments: str) -> Any:
    result = subprocess.run(
        [str(binary), "receiver", *arguments],
        cwd=REPO,
        env={**os.environ, "TIDEX_PRIVATE_ROOT": str(root)},
        capture_output=True,
        text=True,
        timeout=3600,
    )
    if result.returncode:
        raise RuntimeError(f"Rust command rejected: {result.stderr.strip()}")
    return json.loads(result.stdout)


def reference_file(root: Path, name: str, reference: dict[str, str]) -> Path:
    path = root / "references" / f"{name}.json"
    write_new(path, canonical(reference))
    return path


def admission_wire_from_correct_arm(
    request: dict[str, Any], correct_candidate_ref: dict[str, str]
) -> tuple[dict[str, Any], dict[str, Any]]:
    candidate = read_reference(correct_candidate_ref)
    dense_delta = candidate.get("dense_delta")
    if not isinstance(dense_delta, dict):
        raise RuntimeError("correct-arm candidate missing dense_delta; refuse to invent")
    for key in ("path", "sha256", "parameter_count"):
        if key not in dense_delta:
            raise RuntimeError(f"correct-arm dense_delta missing {key}; refuse to invent")
    dense_path = Path(dense_delta["path"])
    if dense_path.is_symlink() or not dense_path.is_file():
        raise RuntimeError("correct-arm dense_delta artifact missing; refuse to invent")
    if sha_file(dense_path) != dense_delta["sha256"]:
        raise RuntimeError("correct-arm dense_delta digest mismatch; refuse to invent")
    if int(dense_delta["parameter_count"]) <= 0:
        raise RuntimeError("correct-arm dense_delta parameter_count invalid")
    basis = read_reference(request["basis"])
    layout = basis.get("layout")
    if not isinstance(layout, dict):
        raise RuntimeError("basis missing parameter_layout; refuse to invent")
    if layout.get("schema") != "tidex.parameter_block_layout/v1":
        raise RuntimeError("basis parameter_layout schema mismatch")
    if int(layout.get("total_parameter_count", -1)) != int(dense_delta["parameter_count"]):
        raise RuntimeError("dense_delta/parameter_layout count mismatch; refuse to invent")
    if candidate.get("basis_sha256") and candidate["basis_sha256"] != request["basis"]["sha256"]:
        raise RuntimeError("candidate basis_sha256 does not match request basis")
    return {
        "path": str(dense_path),
        "sha256": dense_delta["sha256"],
        "parameter_count": int(dense_delta["parameter_count"]),
    }, layout


def plan_value() -> dict[str, Any]:
    """Fixed capacity C, verbalizers, gates — committed before delta construction."""
    # Donor = measured Python ground truth for the same expressions.
    donor_verified = {
        "len({2,2,2})": len({2, 2, 2}),
        "len({1,2,1,2})": len({1, 2, 1, 2}),
        "17 % 5": 17 % 5,
        "10 % 3": 10 % 3,
        "days_in_week": 7,
        "triangle_sides": 3,
        "4+3": 4 + 3,
        "len([1,2,3,4])": len([1, 2, 3, 4]),
        "spider_legs": 8,
        "2+3": 2 + 3,
        "len([9,8,7])": len([9, 8, 7]),
        "car_wheels": 4,
    }
    if donor_verified["len({2,2,2})"] != 1 or donor_verified["len({1,2,1,2})"] != 2:
        raise RuntimeError("donor ground truth inconsistency on set cardinality")
    if donor_verified["17 % 5"] != 2 or donor_verified["10 % 3"] != 1:
        raise RuntimeError("donor ground truth inconsistency on modulo")
    return {
        "schema": SCHEMA,
        "purpose": (
            "semantic behavioral acquisition attempt: Python set-cardinality + modulo; "
            "generation PRE/POST is the claim criterion; margin control is the actuator"
        ),
        "capacity_id": "python.discrete_semantics.set_cardinality_and_modulo.v69",
        "capacity_rationale": (
            "Held-out instruction-following on discrete Python semantics. Virgin "
            "SmolLM2-1.7B-Instruct fails sole-integer answers on these set/modulo "
            "items under chat generation; success requires producing the correct "
            "integer, not matching an arbitrary logit-margin profile."
        ),
        "seed": 20260912,
        "basis_norm": 0.2,
        "flip_margin": 0.35,
        "max_input_tokens": 64,
        "max_new_tokens": 16,
        "donor": {
            "kind": "measured_python_ground_truth",
            "verified_expressions": donor_verified,
            "model_weights_used": False,
        },
        "probes": [
            {
                "probe_id": "C01",
                "user": "Answer with a single integer and nothing else.\nWhat is len({2,2,2}) in Python?",
                "expected": 1,
                "positive_token": "1",
                "negative_token": "3",
            },
            {
                "probe_id": "C02",
                "user": "Answer with a single integer and nothing else.\nWhat is len({1,2,1,2}) in Python?",
                "expected": 2,
                "positive_token": "2",
                "negative_token": "1",
            },
            {
                "probe_id": "C03",
                "user": "Answer with a single integer and nothing else.\nWhat is 17 % 5 in Python?",
                "expected": 2,
                "positive_token": "2",
                "negative_token": "4",
            },
            {
                "probe_id": "C04",
                "user": "Answer with a single integer and nothing else.\nWhat is 10 % 3 in Python?",
                "expected": 1,
                "positive_token": "1",
                "negative_token": "4",
            },
        ],
        "risk_controls": [
            {
                "probe_id": "R01",
                "user": "Answer with a single integer and nothing else.\nHow many days are in a week?",
                "expected": 7,
                "positive_token": "7",
                "negative_token": "5",
            },
            {
                "probe_id": "R02",
                "user": "Answer with a single integer and nothing else.\nHow many sides does a triangle have?",
                "expected": 3,
                "positive_token": "3",
                "negative_token": "4",
            },
            {
                "probe_id": "R03",
                "user": "Answer with a single integer and nothing else.\nWhat is 4+3?",
                "expected": 7,
                "positive_token": "7",
                "negative_token": "8",
            },
            {
                "probe_id": "R04",
                "user": "Answer with a single integer and nothing else.\nWhat is len([1,2,3,4])?",
                "expected": 4,
                "positive_token": "4",
                "negative_token": "3",
            },
        ],
        "unseen_controls": [
            {
                "probe_id": "U01",
                "user": "Answer with a single integer and nothing else.\nHow many legs does a spider have?",
                "expected": 8,
                "positive_token": "8",
                "negative_token": "6",
            },
            {
                "probe_id": "U02",
                "user": "Answer with a single integer and nothing else.\nWhat is 2+3?",
                "expected": 5,
                "positive_token": "5",
                "negative_token": "6",
            },
            {
                "probe_id": "U03",
                "user": "Answer with a single integer and nothing else.\nWhat is len([9,8,7])?",
                "expected": 3,
                "positive_token": "3",
                "negative_token": "4",
            },
            {
                "probe_id": "U04",
                "user": "Answer with a single integer and nothing else.\nHow many wheels does a car have?",
                "expected": 4,
                "positive_token": "4",
                "negative_token": "3",
            },
        ],
        "criteria": {
            "maximum_pre_capacity_pass_rate": 0.25,
            "minimum_post_capacity_pass_rate": 0.75,
            "maximum_control_pass_rate_drop": 0.25,
            "maximum_actual_target_relative_error": 0.05,
            "minimum_correct_wrong_error_ratio": 4.0,
            "maximum_unseen_control_margin_rms_change": 0.25,
        },
        "policy": {
            "schema": "tidex.receiver_compiler_policy/v1",
            "ridge": 1e-10,
            "minimum_decoder_loo_r2": 0.99,
            "minimum_encoder_loo_r2": 0.99,
            "minimum_decoder_loo_cosine": 0.99,
            "maximum_functional_relative_error": 0.05,
            "minimum_identity_margin": 0.1,
            "maximum_quadratic_cost": 0.05,
        },
    }


def score_sole_int(text: str, expected: int) -> dict[str, Any]:
    stripped = text.strip()
    sole = bool(re.fullmatch(r"-?\d+", stripped))
    got = int(stripped) if sole else None
    return {
        "raw": text,
        "sole_integer": sole,
        "parsed": got,
        "expected": expected,
        "pass": sole and got == expected,
    }


def load_model(path: Path, threads: int) -> tuple[Any, Any]:
    torch.set_num_threads(threads)
    torch.use_deterministic_algorithms(True)
    tokenizer = AutoTokenizer.from_pretrained(path, local_files_only=True)
    if tokenizer.pad_token_id is None:
        tokenizer.pad_token = tokenizer.eos_token
    tokenizer.padding_side = "right"
    model = AutoModelForCausalLM.from_pretrained(path, local_files_only=True, torch_dtype=torch.float32)
    model.eval()
    model.requires_grad_(False)
    if type(model).__name__ != "LlamaForCausalLM":
        raise RuntimeError("probe supports the inspected LlamaForCausalLM profile only")
    if any("lora_" in name for name, _ in model.named_parameters()):
        raise RuntimeError("adapter parameters are forbidden")
    model.config.use_cache = False
    return model, tokenizer


def chat_texts(tokenizer: Any, items: list[dict[str, Any]]) -> list[str]:
    texts = []
    for item in items:
        text = tokenizer.apply_chat_template(
            [{"role": "user", "content": item["user"]}],
            tokenize=False,
            add_generation_prompt=True,
        )
        texts.append(text)
    return texts


def single_token_id(tokenizer: Any, token: str) -> int:
    ids = tokenizer.encode(token, add_special_tokens=False)
    if len(ids) != 1:
        raise RuntimeError(f"verbalizer is not a single token: {token!r} -> {ids}")
    return ids[0]


def margins_per_probe(
    model: Any, tokenizer: Any, items: list[dict[str, Any]], max_input_tokens: int
) -> torch.Tensor:
    texts = chat_texts(tokenizer, items)
    encoded = tokenizer(texts, padding=True, truncation=False, return_tensors="pt")
    if encoded.input_ids.shape[1] > max_input_tokens:
        raise RuntimeError("probe input exceeds precommitted token budget")
    positions = encoded.attention_mask.sum(1) - 1
    with torch.no_grad():
        logits = model(**encoded, use_cache=False).logits
        selected = logits[torch.arange(len(items)), positions]
        values = []
        for index, item in enumerate(items):
            pos = single_token_id(tokenizer, item["positive_token"])
            neg = single_token_id(tokenizer, item["negative_token"])
            values.append(selected[index, pos] - selected[index, neg])
        result = torch.stack(values)
    if not torch.isfinite(result).all():
        raise RuntimeError("nonfinite response")
    return result.detach().double().cpu()


def generate_eval(
    model: Any,
    tokenizer: Any,
    items: list[dict[str, Any]],
    max_input_tokens: int,
    max_new_tokens: int,
) -> dict[str, Any]:
    rows = []
    passes = 0
    for item in items:
        text = chat_texts(tokenizer, [item])[0]
        encoded = tokenizer(text, return_tensors="pt")
        if encoded.input_ids.shape[1] > max_input_tokens:
            raise RuntimeError("generation prompt exceeds token budget")
        with torch.no_grad():
            out = model.generate(
                **encoded,
                max_new_tokens=max_new_tokens,
                do_sample=False,
                pad_token_id=tokenizer.eos_token_id,
            )
        raw = tokenizer.decode(out[0][encoded.input_ids.shape[1] :], skip_special_tokens=True).strip()
        scored = score_sole_int(raw, int(item["expected"]))
        scored["probe_id"] = item["probe_id"]
        rows.append(scored)
        passes += int(scored["pass"])
    return {
        "n": len(items),
        "passes": passes,
        "pass_rate": passes / max(len(items), 1),
        "results": rows,
    }


def replay_semantic(model_dir: Path, package_path: Path, output: Path, threads: int) -> None:
    package = json.loads(package_path.read_bytes())
    if sha_file(model_dir / "model.safetensors") != package["checkpoint_sha256"]:
        raise RuntimeError("replay checkpoint identity mismatch")
    for name, expected in package["model_files_sha256"].items():
        if sha_file(model_dir / name) != expected:
            raise RuntimeError("replay tokenizer/config identity mismatch")
    if any("adapter" in path.name or "lora" in path.name for path in model_dir.iterdir()):
        raise RuntimeError("standalone model contains adapter files")
    model, tokenizer = load_model(model_dir, threads)
    items = package["items"]
    margin_values = margins_per_probe(model, tokenizer, items, package["max_input_tokens"])
    generation = generate_eval(
        model, tokenizer, items, package["max_input_tokens"], package["max_new_tokens"]
    )
    write_new(
        output,
        canonical(
            {
                "schema": "tidex.v69_native_forward_replay/v1",
                "checkpoint_sha256": package["checkpoint_sha256"],
                "margins": margin_values.tolist(),
                "generation": generation,
                "model_class": type(model).__name__,
                "lora_parameters_present": False,
                "peft_module_imported": "peft" in sys.modules,
                "model_parameter_count": sum(p.numel() for p in model.parameters()),
            }
        ),
    )


def run(base_dir: Path, binary: Path, root: Path, threads: int) -> dict[str, Any]:
    if root.exists():
        raise RuntimeError("run directory already exists; preserve it rather than repeating measurements")
    if not root.is_absolute() or root == REPO or REPO in root.parents:
        raise RuntimeError("run root must be absolute and outside the checkout")
    root.mkdir(parents=True, mode=0o700)
    with (root / "run.lock").open("xb") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        source_sha = sha_file(SOURCE)
        binary_sha = sha_file(binary)
        reproducibility_bundle = freeze_reproducibility_bundle(root / "reproducibility" / "initial", binary)
        base_file = base_dir / "model.safetensors"
        if sha_file(base_file) != BASE_SHA:
            raise RuntimeError("the exact original SmolLM2-1.7B checkpoint is required")
        files_sha = {name: sha_file(base_dir / name) for name in MODEL_FILES}
        plan = plan_value()
        # Gates and capacity are fixed before interventions. requested_response is
        # derived from PRE margins by the precommitted flip_margin rule only.
        write_new(
            root / "precommit.json",
            canonical(
                {
                    **plan,
                    "base_model_sha256": BASE_SHA,
                    "collector_sha256": source_sha,
                    "tidex_binary_sha256": binary_sha,
                    "model_files_sha256": files_sha,
                    "reproducibility_bundle": reproducibility_bundle,
                    "target_derivation": "requested_i = -baseline_margin_i + flip_margin",
                }
            ),
        )
        model, tokenizer = load_model(base_dir, threads)
        all_items = plan["probes"] + plan["risk_controls"] + plan["unseen_controls"]
        for item in all_items:
            single_token_id(tokenizer, item["positive_token"])
            single_token_id(tokenizer, item["negative_token"])

        print("PRE generation eval (virgin receptor)", flush=True)
        pre_capacity = generate_eval(
            model, tokenizer, plan["probes"], plan["max_input_tokens"], plan["max_new_tokens"]
        )
        pre_risk = generate_eval(
            model, tokenizer, plan["risk_controls"], plan["max_input_tokens"], plan["max_new_tokens"]
        )
        pre_unseen = generate_eval(
            model, tokenizer, plan["unseen_controls"], plan["max_input_tokens"], plan["max_new_tokens"]
        )
        pre_controls = {
            "n": pre_risk["n"] + pre_unseen["n"],
            "passes": pre_risk["passes"] + pre_unseen["passes"],
            "pass_rate": (pre_risk["passes"] + pre_unseen["passes"])
            / max(pre_risk["n"] + pre_unseen["n"], 1),
            "risk": pre_risk,
            "unseen": pre_unseen,
        }
        print(
            f"PRE capacity pass_rate={pre_capacity['pass_rate']:.3f} "
            f"controls={pre_controls['pass_rate']:.3f}",
            flush=True,
        )
        if pre_capacity["pass_rate"] > plan["criteria"]["maximum_pre_capacity_pass_rate"]:
            receipt = {
                "schema": SCHEMA,
                "complete": True,
                "pass": False,
                "stage": "pre_capacity_not_virgin_fail",
                "scope": plan["purpose"],
                "pre_capacity": pre_capacity,
                "pre_controls": pre_controls,
                "claim_boundary": claim_boundary(False, False, False),
            }
            write_new(root / "receipt.json", canonical(receipt))
            return receipt

        baseline = margins_per_probe(model, tokenizer, all_items, plan["max_input_tokens"])
        requested = (-baseline[:4] + float(plan["flip_margin"])).tolist()
        write_new(
            root / "derived_targets.json",
            canonical(
                {
                    "baseline_margins_probes": baseline[:4].tolist(),
                    "flip_margin": plan["flip_margin"],
                    "requested_response": requested,
                    "rule": "requested_i = -baseline_margin_i + flip_margin",
                }
            ),
        )
        print(f"derived requested_response={requested}", flush=True)

        norm_layer = model.model.norm
        original = norm_layer.weight.detach().clone()
        if original.shape != (2048,) or type(norm_layer).__name__ != "LlamaRMSNorm":
            raise RuntimeError("native norm profile differs from the inspected model")

        # Sensitivity: per-probe (hidden * (W_pos - W_neg)) at final RMSNorm.
        texts = chat_texts(tokenizer, plan["probes"])
        encoded = tokenizer(texts, padding=True, return_tensors="pt")
        if encoded.input_ids.shape[1] > plan["max_input_tokens"]:
            raise RuntimeError("sensitivity prompts exceed token budget")
        positions = encoded.attention_mask.sum(1) - 1
        captured: list[torch.Tensor] = []
        handle = norm_layer.register_forward_pre_hook(lambda _module, inputs: captured.append(inputs[0].detach()))
        try:
            with torch.no_grad():
                model(**encoded, use_cache=False)
        finally:
            handle.remove()
        if len(captured) != 1:
            raise RuntimeError("ambiguous native norm observation")
        hidden = captured.pop()[torch.arange(4), positions].float()
        normalized = hidden * torch.rsqrt(hidden.pow(2).mean(-1, keepdim=True) + norm_layer.variance_epsilon)
        sensitivities = []
        for index, item in enumerate(plan["probes"]):
            pos = single_token_id(tokenizer, item["positive_token"])
            neg = single_token_id(tokenizer, item["negative_token"])
            head_difference = model.lm_head.weight[pos] - model.lm_head.weight[neg]
            sensitivities.append((normalized[index] * head_difference).double())
        sensitivities_t = torch.stack(sensitivities)
        if torch.linalg.matrix_rank(sensitivities_t).item() != 4:
            raise RuntimeError("response profile is not independently controllable in four dimensions")
        q, _ = torch.linalg.qr(sensitivities_t.T, mode="reduced")
        axes = (plan["basis_norm"] * q.T).float().contiguous()
        construction = put(
            root,
            {
                "method": "native_final_norm_response_sensitivity_qr",
                "collector_sha256": source_sha,
                "base_model_sha256": BASE_SHA,
                "basis_values_sha256": hashlib.sha256(axes.numpy().tobytes()).hexdigest(),
                "target_values_used": False,
                "semantic_capability_transfer_claim": False,
                "per_probe_verbalizers": True,
            },
        )
        axis_records = []
        for index, axis in enumerate(axes):
            value_reference = put(root, axis.tolist())
            value_file = reference_file(root, f"axis-{index}", value_reference)
            delta = tidex(binary, root, "import-axis", str(value_file))
            axis_records.append({"axis_id": f"norm-axis-{index}", "delta": delta})
        basis = put(
            root,
            {
                "schema": "tidex.receiver_weight_basis/v1",
                "base_model_sha256": BASE_SHA,
                "layout": {
                    "schema": "tidex.parameter_block_layout/v1",
                    "blocks": [
                        {"name": "model.norm.weight", "shape": [2048], "offset": 0, "count": 2048}
                    ],
                    "total_parameter_count": 2048,
                },
                "axes": axis_records,
                "construction_capability_ids": ["receiver.system-identification:v1"],
                "construction_evidence": construction,
            },
        )
        verbalizer_evidence = put(
            root,
            {
                "schema": "tidex.v69_per_probe_verbalizers/v1",
                "chat_template": True,
                "measure": "per_probe_correct_minus_wrong_logit_margin_change",
                "probes": [
                    {
                        "probe_id": item["probe_id"],
                        "user_sha256": hashlib.sha256(item["user"].encode()).hexdigest(),
                        "positive_token": item["positive_token"],
                        "negative_token": item["negative_token"],
                        "positive_token_id": single_token_id(tokenizer, item["positive_token"]),
                        "negative_token_id": single_token_id(tokenizer, item["negative_token"]),
                        "expected": item["expected"],
                    }
                    for item in plan["probes"]
                ],
            },
        )
        # Rust protocol schema is deny-unknown; keep only contracted fields.
        protocol = put(
            root,
            {
                "schema": "tidex.receiver_response_protocol/v1",
                "measure": "next_token_logit_margin_change",
                "base_model_sha256": BASE_SHA,
                "model_config_sha256": files_sha["config.json"],
                "tokenizer_sha256": files_sha["tokenizer.json"],
                "collector_sha256": source_sha,
                "coordinate_ids": [f"probe.{i}" for i in range(4)],
                "prompt_sha256": [
                    hashlib.sha256(item["user"].encode()).hexdigest() for item in plan["probes"]
                ],
                "positive_token_id": single_token_id(tokenizer, plan["probes"][0]["positive_token"]),
                "negative_token_id": single_token_id(tokenizer, plan["probes"][0]["negative_token"]),
                "max_input_tokens": plan["max_input_tokens"],
            },
        )
        write_new(root / "verbalizer_evidence.json", canonical({"reference": verbalizer_evidence}))
        # Broader calibrated support (±0.5..±4 on axes). Does not loosen
        # maximum_quadratic_cost / maximum_functional_relative_error gates.
        axis_rows = []
        for scale in (1.0, 2.0, 4.0):
            axis_rows.append(torch.eye(4) * scale)
            axis_rows.append(-torch.eye(4) * scale)
        axis_rows.append(
            torch.tensor(
                [
                    [0.5, 0.5, 0.5, 0.5],
                    [-0.5, 0.5, -0.5, 0.5],
                    [0.5, -0.5, -0.5, 0.5],
                    [-0.5, -0.5, 0.5, 0.5],
                    [1.0, 1.0, -1.0, -1.0],
                    [-1.0, 1.0, 1.0, -1.0],
                ]
            )
        )
        coordinates = torch.cat(axis_rows).double()
        observation_refs, control_changes = [], []
        try:
            for index, coordinate in enumerate(coordinates):
                with torch.no_grad():
                    norm_layer.weight.copy_(original + (coordinate @ axes.double()).float())
                changed = (
                    margins_per_probe(
                        model, tokenizer, all_items[:8], plan["max_input_tokens"]
                    )
                    - baseline[:8]
                )
                control_changes.append(changed[4:])
                observation_refs.append(
                    put(
                        root,
                        {
                            "schema": "tidex.receiver_response_observation/v1",
                            "observation_id": f"native-norm-{index:03}",
                            "capability_id": "receiver.system-identification:v1",
                            "basis_sha256": basis["sha256"],
                            "protocol_sha256": protocol["sha256"],
                            "receiver_coordinates": coordinate.tolist(),
                            "values": changed[:4].tolist(),
                        },
                    )
                )
                print(f"native calibration {index + 1}/{len(coordinates)}", flush=True)
        finally:
            with torch.no_grad():
                norm_layer.weight.copy_(original)
        if not torch.equal(norm_layer.weight, original):
            raise RuntimeError("temporary norm interventions were not restored")
        controls = torch.stack(control_changes)
        jacobian = (controls[:4] - controls[4:8]).T / 2.0
        metric = jacobian.T @ jacobian / 4.0 + torch.eye(4).double() * 1e-10
        safety_evidence = put(
            root,
            {
                "method": "central_difference_neutral_logit_margin_metric",
                "control_prompt_sha256": [
                    hashlib.sha256(item["user"].encode()).hexdigest()
                    for item in plan["risk_controls"]
                ],
                "control_changes": controls.tolist(),
                "coordinate_jacobian": jacobian.tolist(),
                "global_language_safety_established": False,
            },
        )
        del model, tokenizer, original, hidden, normalized
        gc.collect()

        target = put(
            root,
            {
                "schema": "tidex.functional_response_target/v1",
                "capability_id": "requested.semantic-margin-flip:v1",
                "protocol_sha256": protocol["sha256"],
                "values": requested,
            },
        )
        wrong = put(
            root,
            {
                "schema": "tidex.functional_response_target/v1",
                "capability_id": "wrong.semantic-margin-flip:v1",
                "protocol_sha256": protocol["sha256"],
                "values": [-v for v in requested],
            },
        )
        request = {
            "schema": "tidex.receiver_weight_request/v1",
            "basis": basis,
            "protocol": protocol,
            "target": target,
            "observations": observation_refs,
            "wrong_targets": [wrong],
            "safety": {
                "protected_cortex": {
                    "parameter_importance": metric.diag().tolist(),
                    "directions": [],
                    "max_damage_ratio": 0.15,
                },
                "risk_metric": metric.tolist(),
                "evidence": safety_evidence,
            },
            "policy": plan["policy"],
        }
        return compile_and_eval(
            root,
            binary,
            base_dir,
            plan,
            request,
            baseline,
            files_sha,
            source_sha,
            binary_sha,
            threads,
            pre_capacity,
            pre_controls,
            requested,
        )


def claim_boundary(
    semantic: bool, language: bool, mbpp: bool = False, promote: bool = False
) -> dict[str, Any]:
    return {
        "rust_generated_delta_from_measured_responses": True,
        "lora_delta_source_used": False,
        "receiver_optimizer_steps": 0,
        "backpropagation_used": False,
        "fresh_process_checkpoint_execution": True,
        "donor_model_used": False,
        "donor_kind": "measured_python_ground_truth",
        "new_semantic_capability_transfer_established": bool(semantic),
        "mbpp_transfer_established": bool(mbpp),
        "general_language_preservation_established": bool(language),
        "authorizes_promotion": bool(promote),
    }


def compile_and_eval(
    root: Path,
    binary: Path,
    base_dir: Path,
    plan: dict[str, Any],
    request: dict[str, Any],
    baseline: torch.Tensor,
    files_sha: dict[str, str],
    source_sha: str,
    binary_sha: str,
    threads: int,
    pre_capacity: dict[str, Any],
    pre_controls: dict[str, Any],
    requested: list[float],
) -> dict[str, Any]:
    base_file = base_dir / "model.safetensors"
    all_items = plan["probes"] + plan["risk_controls"] + plan["unseen_controls"]
    target = request["target"]
    wrong = request["wrong_targets"][0]
    results: dict[str, Any] = {}
    for arm, target_ref, negative_ref in (("correct", target, wrong), ("wrong", wrong, target)):
        arm_request = {**request, "target": target_ref, "wrong_targets": [negative_ref]}
        request_ref = put(root, arm_request)
        request_path = reference_file(root, f"initial-request-{arm}", request_ref)
        candidate_ref = tidex(binary, root, "compile", str(request_path))
        candidate_path = reference_file(root, f"initial-candidate-{arm}", candidate_ref)
        candidate = tidex(binary, root, "inspect", str(candidate_path))
        results[arm] = {
            "candidate": candidate_ref,
            "numerical": candidate["numerical"],
            "blockers": candidate["blockers"],
        }
        if candidate["blockers"]:
            return {
                "schema": SCHEMA,
                "complete": True,
                "pass": False,
                "stage": "compiler_rejected",
                "scope": plan["purpose"],
                "arms": results,
                "pre_capacity": pre_capacity,
                "pre_controls": pre_controls,
                "requested_response": requested,
                "claim_boundary": claim_boundary(False, False),
            }
        model_dir = root / "models" / arm
        model_dir.mkdir(parents=True, mode=0o700)
        checkpoint = tidex(
            binary,
            root,
            "materialize",
            str(candidate_path),
            "--base-model",
            str(base_file),
            "--output",
            str(model_dir / "model.safetensors"),
        )
        for name in MODEL_FILES:
            if sha_file(base_dir / name) != files_sha[name]:
                raise RuntimeError("source tokenizer/config changed")
            shutil.copyfile(base_dir / name, model_dir / name)
            (model_dir / name).chmod(0o600)
        package = root / f"replay-package-{arm}.json"
        write_new(
            package,
            canonical(
                {
                    "checkpoint_sha256": checkpoint["materialization"]["output_model_sha256"],
                    "model_files_sha256": files_sha,
                    "items": all_items,
                    "max_input_tokens": plan["max_input_tokens"],
                    "max_new_tokens": plan["max_new_tokens"],
                }
            ),
        )
        replay_path = root / f"replay-{arm}.json"
        subprocess.run(
            [
                sys.executable,
                str(SOURCE),
                "--replay-model",
                str(model_dir),
                "--package",
                str(package),
                "--output",
                str(replay_path),
                "--threads",
                str(threads),
            ],
            check=True,
            timeout=3600,
        )
        replay_payload = json.loads(replay_path.read_bytes())
        replay_margins = torch.tensor(replay_payload["margins"], dtype=torch.float64)
        results[arm].update(
            {
                "checkpoint": checkpoint,
                "actual_response_change": (replay_margins - baseline).tolist(),
                "post_generation": replay_payload["generation"],
                "replay_sha256": sha_file(replay_path),
            }
        )
        print(f"{arm} checkpoint evaluated in fresh process", flush=True)

    requested_t = torch.tensor(requested, dtype=torch.float64)
    correct_change = torch.tensor(results["correct"]["actual_response_change"], dtype=torch.float64)
    wrong_change = torch.tensor(results["wrong"]["actual_response_change"], dtype=torch.float64)
    correct_error = float(
        torch.linalg.vector_norm(correct_change[:4] - requested_t)
        / max(float(torch.linalg.vector_norm(requested_t)), 1e-15)
    )
    wrong_error = float(
        torch.linalg.vector_norm(wrong_change[:4] - requested_t)
        / max(float(torch.linalg.vector_norm(requested_t)), 1e-15)
    )
    ratio = wrong_error / max(correct_error, 1e-15)
    unseen_rms = float(correct_change[8:].square().mean().sqrt())

    post_capacity = {
        "n": 4,
        "passes": sum(
            1
            for row in results["correct"]["post_generation"]["results"][:4]
            if row["pass"]
        ),
        "pass_rate": None,
        "results": results["correct"]["post_generation"]["results"][:4],
    }
    post_capacity["pass_rate"] = post_capacity["passes"] / 4
    post_risk = results["correct"]["post_generation"]["results"][4:8]
    post_unseen = results["correct"]["post_generation"]["results"][8:12]
    post_controls_passes = sum(1 for row in post_risk + post_unseen if row["pass"])
    post_controls = {
        "n": 8,
        "passes": post_controls_passes,
        "pass_rate": post_controls_passes / 8,
        "risk_results": post_risk,
        "unseen_results": post_unseen,
    }
    control_drop = pre_controls["pass_rate"] - post_controls["pass_rate"]
    criteria = plan["criteria"]
    margin_pass = (
        correct_error <= criteria["maximum_actual_target_relative_error"]
        and ratio >= criteria["minimum_correct_wrong_error_ratio"]
        and unseen_rms <= criteria["maximum_unseen_control_margin_rms_change"]
    )
    semantic_pass = (
        pre_capacity["pass_rate"] <= criteria["maximum_pre_capacity_pass_rate"]
        and post_capacity["pass_rate"] >= criteria["minimum_post_capacity_pass_rate"]
        and control_drop <= criteria["maximum_control_pass_rate_drop"]
    )
    language_ok = control_drop <= criteria["maximum_control_pass_rate_drop"]
    # Overall experiment "pass" requires semantic acquisition; margin-only is not enough.
    passed = bool(semantic_pass)
    if sha_file(SOURCE) != source_sha or sha_file(binary) != binary_sha or sha_file(base_file) != BASE_SHA:
        raise RuntimeError("source, compiler or original checkpoint changed during experiment")
    dense_delta, parameter_layout = admission_wire_from_correct_arm(
        request, results["correct"]["candidate"]
    )
    return {
        "schema": SCHEMA,
        "complete": True,
        "pass": passed,
        "stage": "standalone_forward_evaluated",
        "scope": plan["purpose"],
        "capacity_id": plan["capacity_id"],
        "capacity_rationale": plan["capacity_rationale"],
        "precommit_sha256": sha_file(root / "precommit.json"),
        "collector_sha256": source_sha,
        "tidex_binary_sha256": binary_sha,
        "baseline_margins": baseline.tolist(),
        "requested_response": requested,
        "actual_target_relative_error": correct_error,
        "wrong_target_relative_error": wrong_error,
        "correct_wrong_error_ratio": ratio,
        "unseen_control_margin_rms_change": unseen_rms,
        "margin_identification_pass": margin_pass,
        "pre_capacity": pre_capacity,
        "post_capacity": post_capacity,
        "pre_controls": pre_controls,
        "post_controls": post_controls,
        "control_pass_rate_drop": control_drop,
        "criteria": criteria,
        "arms": results,
        "dense_delta": dense_delta,
        "parameter_layout": parameter_layout,
        "claim_boundary": claim_boundary(semantic_pass, language_ok and semantic_pass),
    }


def main() -> int:
    os.umask(0o077)
    parser = argparse.ArgumentParser()
    parser.add_argument("--base-dir", type=Path)
    parser.add_argument("--tidex", type=Path)
    parser.add_argument("--root", type=Path)
    parser.add_argument("--threads", type=int, default=2)
    parser.add_argument("--replay-model", type=Path)
    parser.add_argument("--package", type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    if args.threads < 1 or args.threads > 8:
        parser.error("threads must be 1..8")
    if args.replay_model:
        if not args.package or not args.output:
            parser.error("replay requires package and output")
        replay_semantic(args.replay_model, args.package, args.output, args.threads)
        return 0
    if not args.base_dir or not args.tidex or not args.root:
        parser.error("run requires base-dir, tidex and root")
    existed_before = args.root.exists()
    try:
        result = run(args.base_dir, args.tidex, args.root, args.threads)
    except Exception as error:
        if not existed_before and args.root.exists():
            write_new(
                args.root / "failure.json",
                canonical({"schema": SCHEMA, "complete": False, "error": str(error)}),
            )
        raise
    write_new(args.root / "receipt.json", canonical(result))
    summary = {
        k: v
        for k, v in result.items()
        if k
        not in {
            "arms",
            "baseline_margins",
            "parameter_layout",
            "pre_capacity",
            "post_capacity",
            "pre_controls",
            "post_controls",
        }
    }
    summary["pre_capacity_pass_rate"] = result.get("pre_capacity", {}).get("pass_rate")
    summary["post_capacity_pass_rate"] = result.get("post_capacity", {}).get("pass_rate")
    summary["pre_controls_pass_rate"] = result.get("pre_controls", {}).get("pass_rate")
    summary["post_controls_pass_rate"] = result.get("post_controls", {}).get("pass_rate")
    print(json.dumps(summary, indent=2), flush=True)
    return 0 if result.get("pass") else 2


if __name__ == "__main__":
    raise SystemExit(main())
