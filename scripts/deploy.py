#!/usr/bin/env python3
"""
deploy.py — Build y deploy del auto-updater de Lufal Auxiliar Desktop.

Uso:
  npm run deploy

Requisitos previos (una sola vez):
  1. Generar keypair:
       npx tauri signer generate -w %USERPROFILE%\\.tauri\\lufal.key
  2. Copiar el contenido de lufal.key.pub en src-tauri/tauri.conf.json → plugins.updater.pubkey
  3. Reemplazar PUBLIC_HOST abajo con el dominio/IP pública del servidor
  4. Configurar la variable de entorno TAURI_SIGNING_PRIVATE_KEY:
       PowerShell: $env:TAURI_SIGNING_PRIVATE_KEY = Get-Content ~\\.tauri\\lufal.key -Raw
  5. En el servidor, agregar al Caddyfile:
       route /updates/* {
           root * /var/www/updates
           file_server
       }
     y crear el directorio: mkdir -p /var/www/updates

SSH: usa el alias "cloud-api" definido en ~/.ssh/config (host, user y .pem ya están ahí).
"""

import json
import os
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

# ── Configuración ──────────────────────────────────────────────────────────
SSH_ALIAS   = "cloud-api"                     # alias de ~/.ssh/config
DEPLOY_PATH = "/var/www/updates"              # directorio en el servidor
PUBLIC_HOST = os.getenv("PUBLIC_HOST", "central.pinturaslufal.com")  # URL pública para latest.json
# ───────────────────────────────────────────────────────────────────────────

ROOT = Path(__file__).parent.parent


def run(cmd: list[str], **kwargs) -> subprocess.CompletedProcess:
    print(f"  $ {' '.join(str(c) for c in cmd)}")
    return subprocess.run(cmd, check=True, cwd=ROOT, **kwargs)


def get_version() -> str:
    patch = subprocess.check_output(
        ["git", "rev-list", "--count", "HEAD"],
        text=True,
        cwd=ROOT,
    ).strip()
    return f"0.1.{patch}"


def write_version(version: str) -> None:
    conf_path = ROOT / "src-tauri" / "tauri.conf.json"
    conf = json.loads(conf_path.read_text(encoding="utf-8"))
    conf["version"] = version
    conf_path.write_text(json.dumps(conf, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"  → tauri.conf.json actualizado a v{version}")


def build() -> None:
    print("  $ npm run tauri build -- --bundles nsis")
    subprocess.run(
        "npm run tauri build -- --bundles nsis",
        check=True, cwd=ROOT, shell=True,
    )


def find_artifacts() -> tuple[Path, Path]:
    candidates = [
        ROOT / "src-tauri" / "target" / "x86_64-pc-windows-msvc" / "release" / "bundle" / "nsis",
        ROOT / "src-tauri" / "target" / "release" / "bundle" / "nsis",
    ]
    for bundle_dir in candidates:
        exes = sorted(bundle_dir.glob("*_x64-setup.exe"))
        sigs = sorted(bundle_dir.glob("*_x64-setup.exe.sig"))
        if exes and sigs:
            return exes[0], sigs[0]
    raise FileNotFoundError("No se encontraron artifacts .exe — ¿el build terminó correctamente?")


def safe_name(name: str) -> str:
    return name.replace(" ", "-")


def generate_latest_json(version: str, sig_path: Path, zip_safe: str) -> Path:
    sig_content = sig_path.read_text(encoding="utf-8").strip()
    url = f"https://{PUBLIC_HOST}/updates/v{version}/{zip_safe}"

    manifest = {
        "version": version,
        "notes": "Actualización automática",
        "pub_date": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "platforms": {
            "windows-x86_64": {
                "signature": sig_content,
                "url": url,
            }
        },
    }

    out = ROOT / "latest.json"
    out.write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"  → latest.json generado ({url})")
    return out


def ssh(cmd: str) -> None:
    run(["ssh", SSH_ALIAS, cmd])


def upload(version: str, zip_path: Path, sig_path: Path, zip_safe: str) -> None:
    # Renombrar localmente para eliminar espacios de la URL
    safe_zip = zip_path.parent / zip_safe
    safe_sig = zip_path.parent / (zip_safe + ".sig")
    if zip_path != safe_zip:
        zip_path.rename(safe_zip)
    if sig_path != safe_sig:
        sig_path.rename(safe_sig)

    remote_dir = f"{DEPLOY_PATH}/v{version}"
    ssh(f"mkdir -p {remote_dir}")

    run(["scp", str(safe_zip), f"{SSH_ALIAS}:{remote_dir}/{zip_safe}"])
    run(["scp", str(safe_sig), f"{SSH_ALIAS}:{remote_dir}/{zip_safe}.sig"])
    run(["scp", str(ROOT / "latest.json"), f"{SSH_ALIAS}:{DEPLOY_PATH}/latest.json"])

    print(f"  → Archivos subidos a {SSH_ALIAS}:{DEPLOY_PATH}/")


def check_env() -> None:
    if not os.getenv("TAURI_SIGNING_PRIVATE_KEY"):
        print("ERROR: TAURI_SIGNING_PRIVATE_KEY no está configurado.")
        print()
        print("  PowerShell:")
        print(r"    $env:TAURI_SIGNING_PRIVATE_KEY = Get-Content ~\.tauri\lufal.key -Raw")
        print()
        sys.exit(1)

    if not PUBLIC_HOST:
        print("ERROR: Configura PUBLIC_HOST en scripts/deploy.py (o como env var).")
        sys.exit(1)


def main() -> None:
    print("\n=== Lufal Auxiliar Desktop — Deploy ===\n")

    check_env()

    version = get_version()
    print(f"[1/5] Versión: {version}")

    print("[2/5] Escribiendo versión en tauri.conf.json...")
    write_version(version)

    print("[3/5] Compilando (esto tarda ~5 min)...")
    build()

    print("[4/5] Generando latest.json...")
    zip_path, sig_path = find_artifacts()
    zip_safe = safe_name(zip_path.name)
    generate_latest_json(version, sig_path, zip_safe)

    print("[5/5] Subiendo al servidor via SSH (cloud-api)...")
    upload(version, zip_path, sig_path, zip_safe)

    print(f"\n✓ Deploy completado — v{version}\n")
    print(f"  Manifest:  https://{PUBLIC_HOST}/updates/latest.json")
    print(f"  Installer: https://{PUBLIC_HOST}/updates/v{version}/{zip_safe}\n")


if __name__ == "__main__":
    main()
