#!/usr/bin/env python3
"""Generate hash-bound runtime architecture evidence from Ollama /api/show.

This is intentionally not a fabricated CEREBRO ArchitectureFamilyFingerprint.
It records the exact runtime metadata and tensor manifest that a later Rust
fingerprinting/admission step can consume.
"""

import argparse
import hashlib
import json
import urllib.request


def post_json(url: str, payload: dict) -> dict:
    request = urllib.request.Request(
        url,
        data=json.dumps(payload).encode('utf-8'),
        headers={'Content-Type':'application/json'},
        method='POST',
    )
    with urllib.request.urlopen(request, timeout=120) as response:
        return json.load(response)


def canonical_sha256(value: object) -> str:
    raw = json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode('utf-8')
    return hashlib.sha256(raw).hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument('model')
    parser.add_argument('--endpoint', default='http://127.0.0.1:11434')
    args = parser.parse_args()
    show = post_json(args.endpoint.rstrip('/') + '/api/show', {'model': args.model})
    info = show.get('model_info') or {}
    tensors = show.get('tensors') or []
    architecture = info.get('general.architecture')
    parameter_count = info.get('general.parameter_count')
    if not architecture or not isinstance(parameter_count, int) or parameter_count <= 0 or not tensors:
        print(json.dumps({'status':'failed_closed','reason':'runtime metadata incomplete'}, indent=2))
        return 2
    tensor_manifest = [
        {'name':tensor.get('name'),'shape':tensor.get('shape'),'type':tensor.get('type')}
        for tensor in tensors
    ]
    if any(not row['name'] or not row['shape'] for row in tensor_manifest):
        print(json.dumps({'status':'failed_closed','reason':'tensor manifest incomplete'}, indent=2))
        return 2
    payload = {
        'schema':'cerebro.cross_model.runtime_architecture_evidence/v1',
        'model':args.model,
        'architecture':architecture,
        'parameter_count':parameter_count,
        'quantization':(show.get('details') or {}).get('quantization_level'),
        'model_info':info,
        'tensors':tensor_manifest,
    }
    payload['runtime_metadata_sha256'] = canonical_sha256(payload)
    print(json.dumps(payload, indent=2, ensure_ascii=False))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
