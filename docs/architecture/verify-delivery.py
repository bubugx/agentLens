#!/usr/bin/env python3
"""Read-only verification of the original native-architecture delivery."""

import argparse
import hashlib
import json
import os
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--project-root", type=Path,
                        default=Path(__file__).resolve().parents[2])
    args = parser.parse_args()
    root = args.project_root.resolve()
    manifest = json.loads((root / "docs/architecture/delivery-readback.json").read_text())
    problems = []
    rows = []
    for expected in manifest["originalArtifacts"]:
        relative = Path(expected["projectRelativePath"])
        if relative.is_absolute() or ".." in relative.parts:
            problems.append("invalid project-relative path: " + str(relative))
            continue
        path = root / relative
        row = {"projectRelativePath": str(relative), "absolutePath": str(path),
               "exists": path.is_file(), "isSymlink": path.is_symlink()}
        if path.is_file():
            content = path.read_bytes()
            row.update(sizeBytes=len(content), sha256=hashlib.sha256(content).hexdigest())
            row["matchesReadback"] = (row["sizeBytes"] == expected["sizeBytes"] and
                                      row["sha256"] == expected["sha256"] and
                                      not row["isSymlink"])
        else:
            row["matchesReadback"] = False
        if not row["matchesReadback"]:
            problems.append("missing, changed or linked artifact: " + str(relative))
        rows.append(row)

    evidence_root = root / "docs/architecture/evidence"
    checksums = json.loads((evidence_root / "checksums.json").read_text())
    evidence_problems = []
    for name, digest in checksums["files"].items():
        path = evidence_root / name
        if not path.is_file() or hashlib.sha256(path.read_bytes()).hexdigest() != digest:
            evidence_problems.append(name)
    problems.extend("original evidence checksum mismatch: " + name for name in evidence_problems)
    result = {"projectRoot": str(root), "invocationCwd": os.getcwd(),
              "status": "passed" if not problems else "failed",
              "verifiedArtifactCount": len(rows),
              "originalEvidenceChecksumCount": len(checksums["files"]),
              "artifacts": rows, "problems": problems,
              "scope": "File readback only; host registration and application acceptance are not verified."}
    print(json.dumps(result, ensure_ascii=False, indent=2))
    return 1 if problems else 0


if __name__ == "__main__":
    raise SystemExit(main())
