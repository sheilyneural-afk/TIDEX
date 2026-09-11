#!/usr/bin/env python3
"""Verify the preserved V66 functional-transfer evidence and its physical LoRA bytes.

This does not rerun V66. It cryptographically verifies the preserved evidence and
fails if the receipt does not show the claimed functional gain/control separation.
"""

import hashlib
import json
from pathlib import Path

EVIDENCE = Path('/home/yo/.local/share/tidex/research/v68-response-20260908-a/historical-evidence')
ADAPTER = EVIDENCE / 'v66-adapter'


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open('rb') as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b''):
            h.update(chunk)
    return h.hexdigest()


def main() -> int:
    required = [
        EVIDENCE / 'tidex-v66-receiver-receipt.json',
        ADAPTER / 'manifest.json',
        ADAPTER / 'adapter_model.safetensors',
        ADAPTER / 'adapter_config.json',
    ]
    missing = [str(path) for path in required if not path.is_file()]
    if missing:
        print(json.dumps({'status':'missing_real_artifacts','paths':missing}, indent=2))
        return 2
    receipt = json.loads(required[0].read_text())
    manifest = json.loads(required[1].read_text())
    expected = manifest['adapter']['files_sha256']
    observed = {
        'adapter_model.safetensors': sha256(ADAPTER / 'adapter_model.safetensors'),
        'adapter_config.json': sha256(ADAPTER / 'adapter_config.json'),
    }
    for name, digest in observed.items():
        if expected.get(name) != digest:
            print(json.dumps({'status':'artifact_hash_mismatch','file':name,'expected':expected.get(name),'observed':digest}, indent=2))
            return 2
    metrics = receipt['metrics']
    checks = {
        'receipt_complete': receipt.get('complete') is True,
        'receipt_pass': receipt.get('pass') is True,
        'compiled_gain_positive': metrics['compiled_gain'] > 0.0,
        'wrong_ir_separation_positive': metrics['correct_wrong_pass_rate_gap'] > 0.0,
        'recovered_gain_positive': metrics['recovered_gain'] > 0.0,
        'adapter_hashes_match': True,
        'universal_portability_not_claimed': manifest['claim_boundary']['universal_portability_established'] is False,
    }
    print(json.dumps({
        'schema':'cerebro.cross_model.v66_preserved_evidence_verification/v1',
        'checks':checks,
        'metrics':metrics,
        'adapter_sha256':observed['adapter_model.safetensors'],
        'claim_boundary':manifest['claim_boundary'],
    }, indent=2))
    return 0 if all(checks.values()) else 2


if __name__ == '__main__':
    raise SystemExit(main())
