#!/usr/bin/env python3
"""
deploy.py — Build y deploy del auto-updater de Lufal Auxiliar Desktop.

Uso:
  npm run deploy

La versión se calcula automáticamente:
  - Se busca el último tag v{major}.{minor}.* de la serie activa.
  - Si no existe todavía: patch += 1 desde la versión en los archivos.
  - Si existe: patch += commits nuevos desde ese tag.
  El resultado se escribe en tauri.conf.json y Cargo.toml antes de compilar.
  Al terminar el deploy se crea y pushea el tag vX.Y.Z.

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
import re
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
SSH_ALIAS   = "cloud-api"
DEPLOY_PATH = "/var/www/updates"
PUBLIC_HOST = os.getenv("PUBLIC_HOST", "central.pinturaslufal.com")
# ───────────────────────────────────────────────────────────────────────────

ROOT = Path(__file__).parent.parent


def run(cmd: list[str], **kwargs) -> subprocess.CompletedProcess:
    print(f"  $ {' '.join(str(c) for c in cmd)}")
    return subprocess.run(cmd, check=True, cwd=ROOT, **kwargs)


# ── Versioning automático ──────────────────────────────────────────────────

def read_version_file() -> str:
    conf = json.loads((ROOT / "src-tauri" / "tauri.conf.json").read_text(encoding="utf-8"))
    return conf["version"]


def last_series_tag(major: int, minor: int) -> str | None:
    """Devuelve el tag más reciente de la serie v{major}.{minor}.*, o None."""
    result = subprocess.run(
        ["git", "tag", "--list", f"v{major}.{minor}.*"],
        cwd=ROOT, capture_output=True, text=True,
    )
    tags = [t.strip() for t in result.stdout.splitlines() if t.strip()]
    if not tags:
        return None
    def _key(t: str) -> tuple[int, ...]:
        return tuple(int(p) for p in t.lstrip("v").split("."))
    return max(tags, key=_key)


def commits_since(tag: str) -> int:
    result = subprocess.run(
        ["git", "rev-list", "--count", f"{tag}..HEAD"],
        cwd=ROOT, capture_output=True, text=True, check=True,
    )
    return int(result.stdout.strip())


def calc_new_version() -> tuple[str, str, int]:
    """Devuelve (version_anterior, version_nueva, n_commits)."""
    old = read_version_file()
    major, minor, patch = (int(p) for p in old.split("."))

    tag = last_series_tag(major, minor)

    if tag is None:
        # Primera vez en esta serie — bump de 1 desde el archivo
        n = 1
    else:
        n = commits_since(tag)
        if n == 0:
            print(f"  Aviso: no hay commits nuevos desde {tag}.")
            n = 1  # permite re-deployar el mismo código si es necesario

    new = f"{major}.{minor}.{patch + n}"
    return old, new, n


def write_version(version: str) -> None:
    """Actualiza la versión en tauri.conf.json y Cargo.toml."""
    # tauri.conf.json — reemplaza el campo "version" preservando el formato
    conf_path = ROOT / "src-tauri" / "tauri.conf.json"
    conf_text = conf_path.read_text(encoding="utf-8")
    conf_text = re.sub(
        r'"version"\s*:\s*"[^"]*"',
        f'"version": "{version}"',
        conf_text,
        count=1,
    )
    conf_path.write_text(conf_text, encoding="utf-8")

    # Cargo.toml — reemplaza la primera línea `version = "..."` (sección [package])
    cargo_path = ROOT / "src-tauri" / "Cargo.toml"
    cargo_text = cargo_path.read_text(encoding="utf-8")
    cargo_text = re.sub(
        r'^version\s*=\s*"[^"]*"',
        f'version = "{version}"',
        cargo_text,
        count=1,
        flags=re.MULTILINE,
    )
    cargo_path.write_text(cargo_text, encoding="utf-8")

    print(f"  → tauri.conf.json y Cargo.toml actualizados a {version}")


def commit_and_tag(version: str) -> None:
    """Commitea los archivos de versión, crea el tag y pushea todo."""
    tag = f"v{version}"
    run(["git", "add",
         "src-tauri/tauri.conf.json",
         "src-tauri/Cargo.toml",
         "src-tauri/Cargo.lock"])
    run(["git", "commit", "-m", f"chore: bump versión a {version}"])
    run(["git", "tag", tag])
    run(["git", "push"])
    run(["git", "push", "origin", tag])
    print(f"  → Tag {tag} creado y pusheado")


# ── Build ──────────────────────────────────────────────────────────────────

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


# ── Artifacts y deploy ─────────────────────────────────────────────────────

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


# ── Entrypoint ─────────────────────────────────────────────────────────────

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

    old_version, new_version, n_commits = calc_new_version()
    label = "commit" if n_commits == 1 else "commits"
    print(f"[1/5] Versión: {old_version} → {new_version}  ({n_commits} {label} nuevos)")

    write_version(new_version)

    print("[2/5] Compilando (esto tarda ~5 min)...")
    build()

    print("[3/5] Generando latest.json...")
    zip_path, sig_path = find_artifacts()
    zip_safe = safe_name(zip_path.name)
    generate_latest_json(new_version, sig_path, zip_safe)

    print("[4/5] Subiendo al servidor via SSH (cloud-api)...")
    upload(new_version, zip_path, sig_path, zip_safe)

    print("[5/5] Commiteando versión y creando tag...")
    commit_and_tag(new_version)

    print(f"\n✓ Deploy completado — v{new_version}\n")
    print(f"  Manifest:  https://{PUBLIC_HOST}/updates/latest.json")
    print(f"  Installer: https://{PUBLIC_HOST}/updates/v{new_version}/{zip_safe}\n")


if __name__ == "__main__":
    main()
