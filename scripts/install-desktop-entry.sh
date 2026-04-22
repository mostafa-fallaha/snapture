#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/.." && pwd)"
desktop_template="$repo_root/packaging/snapture.desktop"
desktop_dir="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
desktop_file="$desktop_dir/snapture.desktop"

usage() {
  cat <<'EOF'
Usage: scripts/install-desktop-entry.sh [--binary /absolute/path/to/snapture]

Installs the Snapture desktop entry for the current user and registers it as
the default handler for image/png.
EOF
}

resolve_binary_path() {
  if [[ -n "${1:-}" ]]; then
    printf '%s\n' "$1"
    return 0
  fi

  if command -v snapture >/dev/null 2>&1; then
    command -v snapture
    return 0
  fi

  if [[ -x "$repo_root/target/release/snapture" ]]; then
    printf '%s\n' "$repo_root/target/release/snapture"
    return 0
  fi

  if [[ -x "$repo_root/target/debug/snapture" ]]; then
    printf '%s\n' "$repo_root/target/debug/snapture"
    return 0
  fi

  return 1
}

binary_path=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --binary)
      if [[ $# -lt 2 ]]; then
        echo "Missing value for --binary." >&2 
        usage >&2
        exit 1
      fi
      binary_path="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

if ! binary_path="$(resolve_binary_path "$binary_path")"; then
  echo "Could not find a Snapture binary. Build one first or pass --binary." >&2
  exit 1
fi

if [[ ! -x "$binary_path" ]]; then
  echo "Snapture binary is not executable: $binary_path" >&2
  exit 1
fi

if command -v realpath >/dev/null 2>&1; then
  binary_path="$(realpath "$binary_path")"
fi

if [[ "$binary_path" =~ [[:space:]] ]]; then
  echo "Snapture binary path contains spaces, which breaks xdg-mime desktop lookup: $binary_path" >&2
  echo "Move or symlink the binary to a path without spaces, then re-run the installer." >&2
  exit 1
fi

mkdir -p "$desktop_dir"

sed "s|^Exec=.*$|Exec=$binary_path %f|" "$desktop_template" > "$desktop_file"

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "$desktop_dir" >/dev/null 2>&1 || true
fi

if command -v xdg-mime >/dev/null 2>&1; then
  xdg-mime default snapture.desktop image/png
else
  echo "Installed desktop entry, but xdg-mime is not available to set the PNG default." >&2
fi

if command -v gio >/dev/null 2>&1; then
  gio mime image/png snapture.desktop >/dev/null 2>&1 || true
fi

echo "Installed $desktop_file"
echo "Exec target: $binary_path"
echo "Registered snapture.desktop for image/png"
