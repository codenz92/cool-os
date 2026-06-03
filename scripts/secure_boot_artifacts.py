#!/usr/bin/env python3

import argparse
import hashlib
import os
import shutil
import subprocess
from pathlib import Path


def require_tool(name: str) -> str:
    path = shutil.which(name)
    if not path:
        raise SystemExit(
            f"missing required tool: {name}\n"
            "Install OpenSSL and rerun the Secure Boot target."
        )
    return path


def kernel_hash(path: Path) -> None:
    data = path.read_bytes()
    print(hashlib.sha256(data).hexdigest())


def ensure_cert(out_dir: Path, name: str) -> None:
    key = out_dir / f"{name}.key.pem"
    cert = out_dir / f"{name}.crt.pem"
    if key.exists() and cert.exists():
        return
    openssl = require_tool("openssl")
    subprocess.run(
        [
            openssl,
            "req",
            "-new",
            "-newkey",
            "rsa:2048",
            "-nodes",
            "-x509",
            "-sha256",
            "-days",
            "3650",
            "-subj",
            f"/CN=coolOS Phase 92 {name}/",
            "-keyout",
            str(key),
            "-out",
            str(cert),
        ],
        check=True,
    )


def build_keys(args: argparse.Namespace) -> None:
    out_dir = args.out
    out_dir.mkdir(parents=True, exist_ok=True)
    require_tool("openssl")
    for name in ("PK", "KEK", "db"):
        ensure_cert(out_dir, name)

    if args.vars_template:
        if not args.vars_template.exists():
            raise SystemExit(f"OVMF vars template not found: {args.vars_template}")
        shutil.copyfile(args.vars_template, out_dir / "OVMF_VARS.secboot.fd")

    if args.secure_code and not args.secure_code.exists():
        raise SystemExit(f"OVMF secure code firmware not found: {args.secure_code}")

    status = out_dir / "STATUS.txt"
    status.write_text(
        "\n".join(
            [
                "coolOS Phase 92 Secure Boot test-key artifacts",
                "mode=qemu-secure-fw",
                "pk=PK.crt.pem",
                "kek=KEK.crt.pem",
                "db=db.crt.pem",
                "vars=OVMF_VARS.secboot.fd",
                "loader_integrity=kernel-sha256-embedded",
                "note=OVMF variable enrollment and PE/COFF signing depend on host firmware tooling; this foundation verifies the loader/kernel handoff with an embedded kernel digest.",
                "",
            ]
        )
    )
    print(status)


def main() -> None:
    parser = argparse.ArgumentParser(description="coolOS Secure Boot artifact helper")
    sub = parser.add_subparsers(dest="cmd", required=True)

    hash_parser = sub.add_parser("kernel-hash", help="print SHA-256 for a kernel ELF")
    hash_parser.add_argument("path", type=Path)

    keys_parser = sub.add_parser("keys", help="create local Secure Boot test-key artifacts")
    keys_parser.add_argument("--out", type=Path, required=True)
    keys_parser.add_argument("--vars-template", type=Path)
    keys_parser.add_argument("--secure-code", type=Path)

    args = parser.parse_args()
    if args.cmd == "kernel-hash":
        kernel_hash(args.path)
    elif args.cmd == "keys":
        build_keys(args)


if __name__ == "__main__":
    main()
