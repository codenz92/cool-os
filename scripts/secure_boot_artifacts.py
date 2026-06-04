#!/usr/bin/env python3

import argparse
import hashlib
import os
import shutil
import subprocess
import sys
from pathlib import Path

OWNER_GUID = "c001c0de-93c0-4f6a-9a3d-434f4f4c4653"
ENROLL_REQUIRED_FILES = (
    "PK.cer",
    "KEK.cer",
    "db.cer",
    "PK.esl",
    "KEK.esl",
    "db.esl",
    "auth/PK.auth",
    "auth/KEK.auth",
    "auth/db.auth",
    "auth/dbx.auth",
    "FINGERPRINTS.TXT",
    "README.TXT",
    "SECUREBOOT.TXT",
    "SHA256SUMS",
)


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


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


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


def write_der_cert(src_pem: Path, dst_der: Path) -> None:
    subprocess.run(
        [
            require_tool("openssl"),
            "x509",
            "-in",
            str(src_pem),
            "-outform",
            "DER",
            "-out",
            str(dst_der),
        ],
        check=True,
    )


def cert_fingerprint(cert_pem: Path) -> str:
    der = subprocess.check_output(
        [
            require_tool("openssl"),
            "x509",
            "-in",
            str(cert_pem),
            "-outform",
            "DER",
        ]
    )
    return hashlib.sha256(der).hexdigest()


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
                "enrollment=build-secure-boot-enrollment",
                "",
            ]
        )
    )
    print(status)


def build_enrollment(args: argparse.Namespace) -> None:
    out_dir = args.dir
    enroll = out_dir / "enroll"
    auth_src = out_dir / "auth"
    if not args.loader.exists():
        raise SystemExit(f"signed loader not found: {args.loader}")
    if not auth_src.exists():
        raise SystemExit(f"auth directory not found: {auth_src}")

    enroll.mkdir(parents=True, exist_ok=True)
    enroll_auth = enroll / "auth"
    enroll_auth.mkdir(parents=True, exist_ok=True)

    for name in ("PK", "KEK", "db"):
        cert = out_dir / f"{name}.crt.pem"
        esl = out_dir / f"{name}.esl"
        if not cert.exists():
            raise SystemExit(f"missing certificate: {cert}")
        if not esl.exists():
            raise SystemExit(f"missing ESL: {esl}")
        write_der_cert(cert, enroll / f"{name}.cer")
        shutil.copy2(esl, enroll / f"{name}.esl")

    for auth_name in ("PK.auth", "KEK.auth", "db.auth", "dbx.auth"):
        src = auth_src / auth_name
        if not src.exists():
            raise SystemExit(f"missing auth file: {src}")
        shutil.copy2(src, enroll_auth / auth_name)

    pk_fp = cert_fingerprint(out_dir / "PK.crt.pem")
    kek_fp = cert_fingerprint(out_dir / "KEK.crt.pem")
    db_fp = cert_fingerprint(out_dir / "db.crt.pem")
    loader_fp = sha256_file(args.loader)
    manifest = "\n".join(
        [
            "coolOS Secure Boot manifest",
            "phase=94",
            f"image_build_mode={args.image_mode}",
            f"signed_loader_sha256={loader_fp}",
            f"db_cert_sha256={db_fp}",
            f"kernel_sha256={args.kernel_hash}",
            "enrollment=user-firmware-db",
            "",
        ]
    )
    (enroll / "SECUREBOOT.TXT").write_text(manifest)

    fingerprints = "\n".join(
        [
            "coolOS Phase 94 Secure Boot enrollment fingerprints",
            f"PK.cer sha256={pk_fp}",
            f"KEK.cer sha256={kek_fp}",
            f"db.cer sha256={db_fp}",
            f"BOOTX64.EFI.signed sha256={loader_fp}",
            f"kernel-x86_64 sha256={args.kernel_hash}",
            "",
        ]
    )
    (enroll / "FINGERPRINTS.TXT").write_text(fingerprints)

    readme = "\n".join(
        [
            "coolOS Secure Boot enrollment bundle",
            "",
            "Use this bundle only on firmware that supports custom Secure Boot key enrollment.",
            "Enroll db.cer or db.esl through your firmware UI, then boot coolos-usb-secure.img.",
            "PK/KEK material is included for QEMU/test-key workflows; real firmware usually only needs db.",
            "",
            "Private keys are intentionally excluded from this directory.",
            "Secure Boot must be enabled after enrollment. Microsoft CA/shim/MOK is not used in Phase 94.",
            "",
            "Expected diagnostics after boot:",
            "  hardware",
            "  sysreport",
            "",
        ]
    )
    (enroll / "README.TXT").write_text(readme)

    assert_no_private_keys(enroll)
    write_sha256sums(enroll)
    print(enroll)


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
    if args.enroll:
        verify_enrollment(args.enroll, args.loader, args.kernel_hash)
    if args.usb_image:
        verify_usb_image(args.usb_image)
    print("secure boot artifacts verified")


def assert_no_private_keys(enroll: Path) -> None:
    leaked = []
    for path in enroll.rglob("*"):
        if not path.is_file():
            continue
        name = path.name.lower()
        if name.endswith(".pem") or ".key" in name:
            leaked.append(str(path.relative_to(enroll)))
    if leaked:
        raise SystemExit("private key material must not be in enrollment bundle: " + ", ".join(leaked))


def write_sha256sums(enroll: Path) -> None:
    entries = []
    for path in sorted(enroll.rglob("*")):
        if not path.is_file() or path.name == "SHA256SUMS":
            continue
        rel = path.relative_to(enroll).as_posix()
        entries.append(f"{sha256_file(path)}  {rel}")
    (enroll / "SHA256SUMS").write_text("\n".join(entries) + "\n")


def verify_sha256sums(enroll: Path) -> None:
    sums = enroll / "SHA256SUMS"
    if not sums.exists():
        raise SystemExit(f"missing enrollment SHA256SUMS: {sums}")
    for line in sums.read_text().splitlines():
        if not line.strip():
            continue
        parts = line.split()
        if len(parts) != 2:
            raise SystemExit(f"malformed SHA256SUMS line: {line}")
        expected, rel = parts
        path = enroll / rel
        if not path.exists():
            raise SystemExit(f"SHA256SUMS references missing file: {rel}")
        actual = sha256_file(path)
        if actual != expected:
            raise SystemExit(f"SHA256SUMS mismatch for {rel}: {actual} != {expected}")


def verify_enrollment(enroll: Path, loader: Path, kernel_hash_value: str | None) -> None:
    if not enroll.exists():
        raise SystemExit(f"enrollment bundle not found: {enroll}")
    assert_no_private_keys(enroll)
    missing = [name for name in ENROLL_REQUIRED_FILES if not (enroll / name).exists()]
    if missing:
        raise SystemExit("enrollment bundle missing expected files: " + ", ".join(missing))
    verify_sha256sums(enroll)

    manifest = (enroll / "SECUREBOOT.TXT").read_text()
    loader_fp = sha256_file(loader)
    if f"signed_loader_sha256={loader_fp}" not in manifest:
        raise SystemExit("enrollment manifest signed-loader fingerprint does not match")
    db_fp = hashlib.sha256((enroll / "db.cer").read_bytes()).hexdigest()
    if f"db_cert_sha256={db_fp}" not in manifest:
        raise SystemExit("enrollment manifest db fingerprint does not match")
    if kernel_hash_value and f"kernel_sha256={kernel_hash_value}" not in manifest:
        raise SystemExit("enrollment manifest kernel hash does not match")


def verify_usb_image(image: Path) -> None:
    if not image.exists():
        raise SystemExit(f"secure USB image not found: {image}")
    data = image.read_bytes()
    required = [
        b"coolOS Secure Boot enrollment bundle",
        b"coolOS Secure Boot manifest",
        b"db_cert_sha256=",
        b"signed_loader_sha256=",
        b"kernel_sha256=",
    ]
    missing = [needle.decode("ascii") for needle in required if needle not in data]
    if missing:
        raise SystemExit("secure USB image missing embedded enrollment data: " + ", ".join(missing))
    forbidden = [b"BEGIN PRIVATE KEY", b"BEGIN RSA PRIVATE KEY", b".key.pem"]
    leaked = [needle.decode("ascii") for needle in forbidden if needle in data]
    if leaked:
        raise SystemExit("secure USB image contains private key material: " + ", ".join(leaked))


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
    verify_parser.add_argument("--enroll", type=Path)
    verify_parser.add_argument("--usb-image", type=Path)
    verify_parser.add_argument("--kernel-hash")
    verify_parser.add_argument("--pydeps", type=Path)

    tamper_parser = sub.add_parser("tamper-loader", help="flip a byte in a signed UEFI loader")
    tamper_parser.add_argument("--input", type=Path, required=True)
    tamper_parser.add_argument("--output", type=Path, required=True)

    enroll_parser = sub.add_parser("enrollment", help="build public firmware enrollment bundle")
    enroll_parser.add_argument("--dir", type=Path, required=True)
    enroll_parser.add_argument("--loader", type=Path, required=True)
    enroll_parser.add_argument("--kernel-hash", required=True)
    enroll_parser.add_argument("--image-mode", default="secure-usb-user-enrolled-db")

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
    elif args.cmd == "enrollment":
        build_enrollment(args)


if __name__ == "__main__":
    main()
