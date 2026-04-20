#!/usr/bin/env bash
set -euo pipefail

# Instala entrada .desktop e icone no perfil do usuario (~/.local/share).
# Uso:
#   ./packaging/install-desktop.sh [caminho/para/rpi_open_emulator]
# Se omitido, tenta target/release/rpi_open_emulator e depois target/debug/rpi_open_emulator
# a partir da raiz do repositorio (pai de packaging/).

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
APP_ID="rpi_open_emulator"

canonicalize_path() {
  local p="$1"
  [[ -f "$p" ]] || return 1
  if command -v readlink >/dev/null 2>&1 && readlink -f "$p" >/dev/null 2>&1; then
    readlink -f "$p"
    return
  fi
  python3 -c 'import os,sys; print(os.path.realpath(sys.argv[1]))' "$p"
}

resolve_binary() {
  if [[ -n "${1:-}" ]]; then
    local p="$1"
    [[ -f "$p" ]] || { echo "Erro: binario nao encontrado: $p" >&2; exit 1; }
    canonicalize_path "$p"
    return
  fi
  if [[ -n "${RPI_OPEN_EMULATOR_BIN:-}" ]]; then
    resolve_binary "$RPI_OPEN_EMULATOR_BIN"
    return
  fi
  # Compatibilidade com variavel antiga
  if [[ -n "${RPI5_LAUNCHER_BIN:-}" ]]; then
    resolve_binary "$RPI5_LAUNCHER_BIN"
    return
  fi
  for candidate in "${REPO_ROOT}/target/release/${APP_ID}" "${REPO_ROOT}/target/debug/${APP_ID}"; do
    if [[ -f "$candidate" ]]; then
      canonicalize_path "$candidate"
      return
    fi
  done
  echo "Erro: nao encontrei ${APP_ID} em target/release nem target/debug." >&2
  echo "Compile com: cargo build --release" >&2
  echo "Ou passe o caminho: $0 /caminho/completo/${APP_ID}" >&2
  exit 1
}

BIN="$(resolve_binary "${1:-}")"
ICON_SRC="${SCRIPT_DIR}/${APP_ID}.svg"
DESKTOP_IN="${SCRIPT_DIR}/${APP_ID}.desktop.in"

[[ -f "$ICON_SRC" ]] || { echo "Erro: icone nao encontrado: $ICON_SRC" >&2; exit 1; }
[[ -f "$DESKTOP_IN" ]] || { echo "Erro: template .desktop nao encontrado: $DESKTOP_IN" >&2; exit 1; }

DATA_HOME="${XDG_DATA_HOME:-$HOME/.local/share}"
APPS_DIR="${DATA_HOME}/applications"
ICONS_SCALABLE="${DATA_HOME}/icons/hicolor/scalable/apps"
mkdir -p "$APPS_DIR" "$ICONS_SCALABLE"

install -m0644 "$ICON_SRC" "${ICONS_SCALABLE}/${APP_ID}.svg"

TMP_DESKTOP="$(mktemp)"
python3 -c '
import pathlib, sys
bin_path = pathlib.Path(sys.argv[1]).resolve()
text = pathlib.Path(sys.argv[2]).read_text(encoding="utf-8")
pathlib.Path(sys.argv[3]).write_text(text.replace("@EXEC@", str(bin_path)), encoding="utf-8")
' "$BIN" "$DESKTOP_IN" "$TMP_DESKTOP"
install -m0644 "$TMP_DESKTOP" "${APPS_DIR}/${APP_ID}.desktop"
rm -f "$TMP_DESKTOP"

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -f -t "${DATA_HOME}/icons/hicolor" 2>/dev/null || true
fi
if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "$APPS_DIR" 2>/dev/null || true
fi

echo "Instalado:"
echo "  ${APPS_DIR}/${APP_ID}.desktop"
echo "  ${ICONS_SCALABLE}/${APP_ID}.svg"
echo "  Exec=${BIN}"
echo "Abra o menu de aplicativos e procure por \"RPI Open Emulator\" (pode precisar sair e entrar na sessao)."
