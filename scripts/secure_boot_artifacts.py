#!/usr/bin/env python3

import argparse
import hashlib
import os
import shutil
import subprocess
import sys
from pathlib import Path

OWNER_GUID = "c001c0de-93c0-4f6a-9a3d-434f4f4c4653"


def find_tool(name: str) -> str | None:
    return shutil.which(name)


def require_tool(name: str) -> str:
    path = shutil.which(name)
    if not path:
        raise SystemExit(
            f"missing required tool: {name}\n"
            "Install the required Secure Boot host tools and rerun the target."
        )
    return path


def require_signing_backend() -> str:
    if find_tool("sbsign") and find_tool("sbverify"):
        return "sbsign"
    if find_tool("osslsigncode"):
        return "osslsigncode"
    raise SystemExit(
        "missing PE/COFF signing backend\n"
        "Install sbsigntools (sbsign/sbverify) on Linux, or install osslsigncode on macOS:\n"
        "  brew install osslsigncode\n"
    )


def ensure_pydeps(pydeps: Path | None) -> str:
    if pydeps is None:
        return os.environ.get("PYTHONPATH", "")
    pydeps.mkdir(parents=True, exist_ok=True)
    env_path = str(pydeps)

    def imports_ok() -> bool:
        code = (
            "import cryptography;"
            "import pefile;"
            "import virt.firmware.vars;"
            "import virt.firmware.sigdb;"
            "import virt.firmware.varstore.edk2"
        )
        env = os.environ.copy()
        env["PYTHONPATH"] = env_path + os.pathsep + env.get("PYTHONPATH", "")
        return (
            subprocess.run(
                [sys.executable, "-c", code],
                env=env,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            ).returncode
            == 0
        )

    if imports_ok():
        return env_path

    subprocess.run(
        [
            sys.executable,
            "-m",
            "pip",
            "install",
            "--target",
            str(pydeps),
            "--upgrade",
            "cryptography",
            "pefile",
        ],
        check=True,
    )
    subprocess.run(
        [
            sys.executable,
            "-m",
            "pip",
            "install",
            "--target",
            str(pydeps),
            "--upgrade",
            "--no-deps",
            "virt-firmware",
        ],
        check=True,
    )
    if not imports_ok():
        raise SystemExit(
            "failed to prepare local Secure Boot Python helpers under "
            f"{pydeps}"
        )
    return env_path


def pydeps_env(pydeps_path: str) -> dict[str, str]:
    env = os.environ.copy()
    if pydeps_path:
        env["PYTHONPATH"] = pydeps_path + os.pathsep + env.get("PYTHONPATH", "")
    return env


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
            f"/CN=coolOS Phase 93 {name}/",
            "-keyout",
            str(key),
            "-out",
            str(cert),
        ],
        check=True,
    )


def write_sigdb(pydeps_path: str, out_dir: Path, name: str, cert: Path) -> None:
    subprocess.run(
        [
            sys.executable,
            "-m",
            "virt.firmware.sigdb",
            "-o",
            str(out_dir / f"{name}.esl"),
            "--add-cert",
            OWNER_GUID,
            str(cert),
        ],
        env=pydeps_env(pydeps_path),
        check=True,
    )


def enroll_vars(pydeps_path: str, args: argparse.Namespace) -> None:
    if not args.vars_template:
        raise SystemExit("missing --vars-template for enrolled Secure Boot vars image")
    if not args.vars_template.exists():
        raise SystemExit(f"OVMF vars template not found: {args.vars_template}")

    vars_out = args.out / "OVMF_VARS.secboot.fd"
    auth_dir = args.out / "auth"
    auth_dir.mkdir(parents=True, exist_ok=True)

    subprocess.run(
        [
            sys.executable,
            "-m",
            "virt.firmware.vars",
            "-i",
            str(args.vars_template),
            "-o",
            str(vars_out),
            "--set-pk",
            OWNER_GUID,
            str(args.out / "PK.crt.pem"),
            "--add-kek",
            OWNER_GUID,
            str(args.out / "KEK.crt.pem"),
            "--add-db",
            OWNER_GUID,
            str(args.out / "db.crt.pem"),
            "--sb",
        ],
        env=pydeps_env(pydeps_path),
        check=True,
    )
    subprocess.run(
        [
            sys.executable,
            "-m",
            "virt.firmware.vars",
            "-i",
            str(vars_out),
            "--output-auth",
            str(auth_dir),
        ],
        env=pydeps_env(pydeps_path),
        check=True,
    )


def build_keys(args: argparse.Namespace) -> None:
    out_dir = args.out
    out_dir.mkdir(parents=True, exist_ok=True)
    require_tool("openssl")
    backend = require_signing_backend()
    pydeps_path = ensure_pydeps(args.pydeps)
    for name in ("PK", "KEK", "db"):
        ensure_cert(out_dir, name)
        write_sigdb(pydeps_path, out_dir, name, out_dir / f"{name}.crt.pem")

    enroll_vars(pydeps_path, args)

    if args.secure_code and not args.secure_code.exists():
        raise SystemExit(f"OVMF secure code firmware not found: {args.secure_code}")

    status = out_dir / "STATUS.txt"
    status.write_text(
        "\n".join(
            [
                "coolOS Phase 93 Secure Boot test-key artifacts",
                "mode=qemu-secure-fw-enforced",
                "pk=PK.crt.pem",
                "kek=KEK.crt.pem",
                "db=db.crt.pem",
                "vars=OVMF_VARS.secboot.fd",
                "auth=auth/",
                "esl=PK.esl,KEK.esl,db.esl",
                f"owner_guid={OWNER_GUID}",
                f"signing_backend={backend}",
                "loader_integrity=pe-coff-authenticode",
                "kernel_integrity=sha256-embedded",
                "enforcement=ovmf-secure-boot",
                "",
            ]
        )
    )
    print(status)


def sign_loader(args: argparse.Namespace) -> None:
    args.output.parent.mkdir(parents=True, exist_ok=True)
    if args.output.exists():
        args.output.unlink()
    backend = require_signing_backend()
    if backend == "sbsign":
        subprocess.run(
            [
                require_tool("sbsign"),
                "--key",
                str(args.key),
                "--cert",
                str(args.cert),
                "--output",
                str(args.output),
                str(args.input),
            ],
            check=True,
        )
    else:
        subprocess.run(
            [
                require_tool("osslsigncode"),
                "sign",
                "-h",
                "sha256",
                "-certs",
                str(args.cert),
                "-key",
                str(args.key),
                "-n",
                "coolOS UEFI loader",
                "-in",
                str(args.input),
                "-out",
                str(args.output),
            ],
            check=True,
        )
    verify_loader(args.output, args.cert)
    print(args.output)


def verify_loader(path: Path, cert: Path) -> None:
    if find_tool("sbverify"):
        subprocess.run([require_tool("sbverify"), "--cert", str(cert), str(path)], check=True)
        return
    if find_tool("osslsigncode"):
        subprocess.run(
            [
                require_tool("osslsigncode"),
                "verify",
                "-CAfile",
                str(cert),
                "-in",
                str(path),
            ],
            check=True,
        )
        return
    raise SystemExit(
        "missing signature verification backend\n"
        "Install sbverify or osslsigncode."
    )


def verify_artifacts(args: argparse.Namespace) -> None:
    pydeps_path = ensure_pydeps(args.pydeps)
    verify_loader(args.loader, args.dir / "db.crt.pem")
    output = subprocess.check_output(
        [
            sys.executable,
            "-m",
            "virt.firmware.vars",
            "-i",
            str(args.vars),
            "-p",
        ],
        env=pydeps_env(pydeps_path),
        text=True,
    )
    required = [
        "SecureBootEnable    : bool: ON",
        "PK                  : blob:",
        "KEK                 : blob:",
        "db                  : blob:",
        "dbx                 : blob:",
    ]
    missing = [item for item in required if item not in output]
    if missing:
        raise SystemExit("Secure Boot varstore missing expected entries: " + ", ".join(missing))
    print("secure boot artifacts verified")


def tamper_loader(args: argparse.Namespace) -> None:
    data = bytearray(args.input.read_bytes())
    if len(data) < 1024:
        raise SystemExit(f"refusing to tamper unexpectedly small EFI binary: {args.input}")
    offset = min(0x200, len(data) - 1)
    data[offset] ^= 0x01
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_bytes(data)
    print(args.output)


def main() -> None:
    parser = argparse.ArgumentParser(description="coolOS Secure Boot artifact helper")
    sub = parser.add_subparsers(dest="cmd", required=True)

    hash_parser = sub.add_parser("kernel-hash", help="print SHA-256 for a kernel ELF")
    hash_parser.add_argument("path", type=Path)

    keys_parser = sub.add_parser("keys", help="create local Secure Boot test-key artifacts")
    keys_parser.add_argument("--out", type=Path, required=True)
    keys_parser.add_argument("--vars-template", type=Path)
    keys_parser.add_argument("--secure-code", type=Path)
    keys_parser.add_argument("--pydeps", type=Path)

    sign_parser = sub.add_parser("sign-loader", help="sign a UEFI PE/COFF loader")
    sign_parser.add_argument("--input", type=Path, required=True)
    sign_parser.add_argument("--output", type=Path, required=True)
    sign_parser.add_argument("--cert", type=Path, required=True)
    sign_parser.add_argument("--key", type=Path, required=True)

    verify_parser = sub.add_parser("verify", help="verify signed loader and enrolled vars")
    verify_parser.add_argument("--dir", type=Path, required=True)
    verify_parser.add_argument("--loader", type=Path, required=True)
    verify_parser.add_argument("--vars", type=Path, required=True)
    verify_parser.add_argument("--pydeps", type=Path)

    tamper_parser = sub.add_parser("tamper-loader", help="flip a byte in a signed UEFI loader")
    tamper_parser.add_argument("--input", type=Path, required=True)
    tamper_parser.add_argument("--output", type=Path, required=True)

    args = parser.parse_args()
    if args.cmd == "kernel-hash":
        kernel_hash(args.path)
    elif args.cmd == "keys":
        build_keys(args)
    elif args.cmd == "sign-loader":
        sign_loader(args)
    elif args.cmd == "verify":
        verify_artifacts(args)
    elif args.cmd == "tamper-loader":
        tamper_loader(args)


if __name__ == "__main__":
    main()
