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

import getpass
import json
import os
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

sys.stdout.reconfigure(encoding="utf-8")
sys.stderr.reconfigure(encoding="utf-8")


def load_dotenv() -> None:
    env_path = Path(__file__).parent.parent / ".env"
    if not env_path.exists():
        return
    for line in env_path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, _, value = line.partition("=")
        os.environ.setdefault(key.strip(), value.strip())


load_dotenv()

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
    conf_path = ROOT / "src-tauri" / "tauri.conf.json"
    conf = json.loads(conf_path.read_text(encoding="utf-8"))
    return conf["version"]


def build() -> None:
    env = os.environ.copy()

    if not env.get("TAURI_SIGNING_PRIVATE_KEY_PASSWORD"):
        env["TAURI_SIGNING_PRIVATE_KEY_PASSWORD"] = getpass.getpass("  Contraseña de la llave privada: ")

    while True:
        print("  $ npm run tauri build -- --bundles nsis")
        result = subprocess.run("npm run tauri build -- --bundles nsis", cwd=ROOT, shell=True, env=env)
        if result.returncode == 0:
            return
        answer = input("\n  ¿Contraseña incorrecta? Intentar de nuevo [s/N]: ").strip().lower()
        if answer == "s":
            env["TAURI_SIGNING_PRIVATE_KEY_PASSWORD"] = getpass.getpass("  Contraseña de la llave privada: ")
        else:
            sys.exit(result.returncode)


def find_artifacts() -> tuple[Path, Path]:
    candidates = [
        ROOT / "src-tauri" / "target" / "x86_64-pc-windows-msvc" / "release" / "bundle" / "nsis",
        ROOT / "src-tauri" / "target" / "release" / "bundle" / "nsis",
    ]
    for bundle_dir in candidates:
        exes = sorted(bundle_dir.glob("*_x64-setup.exe"), key=lambda p: p.stat().st_mtime)
        sigs = sorted(bundle_dir.glob("*_x64-setup.exe.sig"), key=lambda p: p.stat().st_mtime)
        if exes and sigs:
            return exes[-1], sigs[-1]
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
        zip_path.replace(safe_zip)
    if sig_path != safe_sig:
        sig_path.replace(safe_sig)

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
    print(f"[1/4] Versión: {version}")

    print("[2/4] Compilando (esto tarda ~5 min)...")
    build()

    print("[3/4] Generando latest.json...")
    zip_path, sig_path = find_artifacts()
    zip_safe = safe_name(zip_path.name)
    generate_latest_json(version, sig_path, zip_safe)

    print("[4/4] Subiendo al servidor via SSH (cloud-api)...")
    upload(version, zip_path, sig_path, zip_safe)

    print(f"\n✓ Deploy completado — v{version}\n")
    print(f"  Manifest:  https://{PUBLIC_HOST}/updates/latest.json")
    print(f"  Installer: https://{PUBLIC_HOST}/updates/v{version}/{zip_safe}\n")


if __name__ == "__main__":
    main()
