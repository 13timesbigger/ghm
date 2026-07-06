#!/usr/bin/env sh
set -eu

BIN_NAME="ghad"
PACKAGE_NAME="ghm-cli"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
GHM_REPO="${GHM_REPO:-corelmax/github-monitor}"
GHM_REF="${GHM_REF:-main}"

usage() {
  cat <<EOF
Install Github Activity to Agents Dispatcher (GHAAD) as ${BIN_NAME}.

Usage:
  ./install.sh [--dir DIR] [--debug] [--no-build]
  curl -fsSL https://raw.githubusercontent.com/corelmax/github-monitor/main/install.sh | sh

Options:
  --dir DIR    Install the binary into DIR (default: ${INSTALL_DIR})
  --debug      Install the debug build from target/debug
  --no-build   Skip cargo build and install the existing binary
  -h, --help   Show this help

Environment:
  INSTALL_DIR  Install directory used when --dir is not provided
  GHM_REPO     GitHub repository to download when run outside a checkout
               (default: ${GHM_REPO})
  GHM_REF      Branch, tag, or commit to download when run outside a checkout
               (default: ${GHM_REF})
EOF
}

profile="release"
build=1
tmpdir=""

cleanup() {
  if [ -n "$tmpdir" ] && [ -d "$tmpdir" ]; then
    rm -rf "$tmpdir"
  fi
}
trap cleanup EXIT INT TERM

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

source_dir="$(pwd)"
if [ ! -f "${source_dir}/Cargo.toml" ] || [ ! -d "${source_dir}/crates/${PACKAGE_NAME}" ]; then
  if [ "$build" -eq 0 ]; then
    echo "error: --no-build can only be used from a ${GHM_REPO} checkout" >&2
    exit 1
  fi

  if command -v mktemp >/dev/null 2>&1; then
    tmpdir="$(mktemp -d "${TMPDIR:-/tmp}/ghad-install.XXXXXX")"
  else
    tmpdir="${TMPDIR:-/tmp}/ghad-install.$$"
    mkdir -p "$tmpdir"
  fi

  archive="${tmpdir}/source.tar.gz"
  archive_url="https://codeload.github.com/${GHM_REPO}/tar.gz/${GHM_REF}"
  echo "Downloading ${GHM_REPO}@${GHM_REF}..."

  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$archive_url" -o "$archive"
  elif command -v wget >/dev/null 2>&1; then
    wget -qO "$archive" "$archive_url"
  else
    echo "error: curl or wget is required to download ${GHM_REPO}" >&2
    exit 1
  fi

  tar -xzf "$archive" -C "$tmpdir" --strip-components 1
  source_dir="$tmpdir"
fi

if [ "$build" -eq 1 ]; then
  if [ "$profile" = "release" ]; then
    (cd "$source_dir" && cargo build --release --package "$PACKAGE_NAME")
  else
    (cd "$source_dir" && cargo build --package "$PACKAGE_NAME")
  fi
fi

binary_path="${source_dir}/target/${profile}/${BIN_NAME}"
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
