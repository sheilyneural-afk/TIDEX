#!/usr/bin/env python3
"""Real physical V67 materialization smoke using preserved V66 LoRA bytes.

Default mode validates every source artifact. `--materialize` additionally runs
CEREBRO3's Rust weight actuator to create a standalone candidate checkpoint in a
fresh private temporary root. No prompt steering or synthetic delta is used.
"""

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent
EVIDENCE = Path('/home/yo/.local/share/tidex/research/v68-response-20260908-a/historical-evidence')
ADAPTER = EVIDENCE / 'v66-adapter'
BASE = Path('/home/yo/.cache/huggingface/hub/models--HuggingFaceTB--SmolLM2-1.7B-Instruct/snapshots/31b70e2e869a7173562077fd711b654946d38674/model.safetensors')
EXPECTED_BASE_SHA = 'f55217be716b6a997b97b9d8d7eb6fad02e00858f5010ec24f64603c3a98a0e8'


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open('rb') as handle:
        for chunk in iter(lambda: handle.read(4 * 1024 * 1024), b''):
            h.update(chunk)
    return h.hexdigest()


def validate_sources() -> dict:
    manifest_path = ADAPTER / 'manifest.json'
    required = [BASE, manifest_path, ADAPTER/'adapter_model.safetensors', ADAPTER/'adapter_config.json']
    missing = [str(path) for path in required if not path.exists()]
    if missing:
        raise RuntimeError(f'missing real artifacts: {missing}')
    manifest = json.loads(manifest_path.read_text())
    if manifest['receiver']['weight_file_sha256'] != EXPECTED_BASE_SHA:
        raise RuntimeError('manifest base-model identity mismatch')
    # The base checkpoint is a HF symlink whose target blob name is already its SHA-256.
    resolved = BASE.resolve()
    if resolved.name != EXPECTED_BASE_SHA:
        observed = sha256(BASE)
        if observed != EXPECTED_BASE_SHA:
            raise RuntimeError(f'base model hash mismatch: {observed}')
    observed_adapter = sha256(ADAPTER/'adapter_model.safetensors')
    observed_config = sha256(ADAPTER/'adapter_config.json')
    files = manifest['adapter']['files_sha256']
    if observed_adapter != files['adapter_model.safetensors'] or observed_config != files['adapter_config.json']:
        raise RuntimeError('PEFT artifact hash mismatch')
    return {
        'base_model': str(BASE),
        'base_sha256': EXPECTED_BASE_SHA,
        'adapter_sha256': observed_adapter,
        'adapter_config_sha256': observed_config,
        'capability': manifest['capability'],
    }


def materialize() -> dict:
    binary = Path('/tmp/tidex-cargo-target/debug/v67-weight-actuator-smoke')
    if not binary.is_file():
        subprocess.run([
            'cargo','build','--manifest-path',str(ROOT/'Cargo.toml'),'--bin','v67-weight-actuator-smoke','--offline'
        ], check=True)
    temp = Path(tempfile.mkdtemp(prefix='cerebro3-v67-real-', dir='/tmp'))
    os.chmod(temp, 0o700)
    output = temp/'candidate.safetensors'
    receipt = temp/'receipt.json'
    layout = temp/'layout.json'
    delta = temp/'delta-ref.json'
    env = os.environ.copy()
    env['TIDEX_PRIVATE_ROOT'] = str(temp)
    subprocess.run([
        str(binary),
        '--private-root', str(temp),
        '--base-model', str(BASE),
        '--adapter-dir', str(ADAPTER),
        '--manifest', str(ADAPTER/'manifest.json'),
        '--output-model', str(output),
        '--receipt', str(receipt),
        '--layout-output', str(layout),
        '--delta-reference-output', str(delta),
    ], check=True, env=env)
    data = json.loads(receipt.read_text())
    if not output.is_file() or not data['claim_boundary']['rust_direct_weight_actuation_established']:
        raise RuntimeError('physical materialization receipt invalid')
    return {
        'temporary_root': str(temp),
        'candidate': str(output),
        'candidate_sha256': sha256(output),
        'receipt': data,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument('--materialize', action='store_true')
    args = parser.parse_args()
    try:
        result = {'schema':'cerebro.cross_model.real_transfer_smoke/v1','sources':validate_sources()}
        if args.materialize:
            result['materialization'] = materialize()
        else:
            result['materialization'] = 'not_requested; pass --materialize to execute the physical Rust actuator'
        print(json.dumps(result, indent=2))
        return 0
    except Exception as error:
        print(json.dumps({'status':'failed_closed','error':str(error)}, indent=2))
        return 2


if __name__ == '__main__':
    raise SystemExit(main())
