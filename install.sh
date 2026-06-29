#!/usr/bin/env sh
set -eu

BIN_NAME="ghm"
PACKAGE_NAME="ghm-cli"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"

usage() {
  cat <<EOF
Install GitHub Monitor (${BIN_NAME}).

Usage:
  ./install.sh [--dir DIR] [--debug] [--no-build]

Options:
  --dir DIR    Install the binary into DIR (default: ${INSTALL_DIR})
  --debug      Install the debug build from target/debug
  --no-build   Skip cargo build and install the existing binary
  -h, --help   Show this help

Environment:
  INSTALL_DIR  Install directory used when --dir is not provided
EOF
}

profile="release"
build=1

while [ "$#" -gt 0 ]; do
  case "$1" in
    --dir)
      if [ "$#" -lt 2 ]; then
        echo "error: --dir requires a value" >&2
        exit 2
      fi
      INSTALL_DIR="$2"
      shift 2
      ;;
    --debug)
      profile="debug"
      shift
      ;;
    --no-build)
      build=0
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if ! command -v cargo >/dev/null 2>&1; then
  echo "error: cargo is required to build ${BIN_NAME}" >&2
  echo "Install Rust from https://rustup.rs/ and run this script again." >&2
  exit 1
fi

if [ "$build" -eq 1 ]; then
  if [ "$profile" = "release" ]; then
    cargo build --release --package "$PACKAGE_NAME"
  else
    cargo build --package "$PACKAGE_NAME"
  fi
fi

binary_path="target/${profile}/${BIN_NAME}"
if [ ! -x "$binary_path" ]; then
  echo "error: ${binary_path} does not exist or is not executable" >&2
  echo "Run cargo build first or omit --no-build." >&2
  exit 1
fi

mkdir -p "$INSTALL_DIR"
destination="${INSTALL_DIR}/${BIN_NAME}"

if [ -w "$INSTALL_DIR" ]; then
  install -m 0755 "$binary_path" "$destination"
else
  echo "Installing to ${INSTALL_DIR} requires elevated permissions."
  sudo install -m 0755 "$binary_path" "$destination"
fi

echo "Installed ${BIN_NAME} to ${destination}"
if command -v "$BIN_NAME" >/dev/null 2>&1; then
  "$BIN_NAME" --version || true
else
  echo "Add ${INSTALL_DIR} to your PATH to run '${BIN_NAME}' from anywhere."
fi
