#!/usr/bin/env bash
# Install FVA from GitHub Releases.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/Xeon-Dot/fva/main/scripts/install.sh | bash
#   ./scripts/install.sh [--version v0.2.0] [--install-dir ~/.local/bin]
#
# Environment:
#   FVA_VERSION    Pin release tag (e.g. v0.2.0)
#   INSTALL_DIR    Destination directory (default: ~/.local/bin)
#   FVA_REPO       GitHub repo slug (default: Xeon-Dot/fva)

set -euo pipefail

REPO="${FVA_REPO:-Xeon-Dot/fva}"
BINARY="fva"
INSTALL_DIR="${INSTALL_DIR:-${HOME}/.local/bin}"
VERSION="${FVA_VERSION:-}"

usage() {
  cat <<'EOF'
Install FVA from GitHub Releases.

Usage:
  install.sh [options]

Options:
  -v, --version <tag>       Install a specific release (e.g. v0.2.0)
  -d, --install-dir <path>  Install directory (default: ~/.local/bin)
  -h, --help                Show this help

Environment:
  FVA_VERSION    Same as --version
  INSTALL_DIR    Same as --install-dir
  FVA_REPO       GitHub repository slug
EOF
}

log() {
  printf '==> %s\n' "$*"
}

die() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

have_cmd() {
  command -v "$1" >/dev/null 2>&1
}

parse_args() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      -v | --version)
        [[ $# -ge 2 ]] || die "missing value for $1"
        VERSION="$2"
        shift 2
        ;;
      -d | --install-dir)
        [[ $# -ge 2 ]] || die "missing value for $1"
        INSTALL_DIR="$2"
        shift 2
        ;;
      -h | --help)
        usage
        exit 0
        ;;
      *)
        die "unknown argument: $1 (try --help)"
        ;;
    esac
  done
}

detect_platform() {
  local os arch

  os="$(uname -s)"
  arch="$(uname -m)"

  case "${os}" in
    Linux) OS="linux" ;;
    Darwin) OS="macos" ;;
    *) die "unsupported operating system: ${os} (Linux and macOS only)" ;;
  esac

  case "${arch}" in
    x86_64 | amd64) ARCH="amd64" ;;
    aarch64 | arm64) ARCH="arm64" ;;
    *) die "unsupported CPU architecture: ${arch}" ;;
  esac

  if [[ "${OS}" == "macos" && "${ARCH}" == "amd64" ]]; then
    die "no pre-built Intel Mac binary is published yet; build from source with: cargo install --git https://github.com/${REPO}"
  fi

  ARTIFACT="fva-${OS}-${ARCH}"
  ARCHIVE_EXT="tar.gz"
}

download() {
  local url="$1"
  local dest="$2"

  if have_cmd curl; then
    curl -fsSL --retry 3 --retry-delay 1 -o "${dest}" "${url}"
  elif have_cmd wget; then
    wget -qO "${dest}" "${url}"
  else
    die "curl or wget is required"
  fi
}

asset_url() {
  local file="$1"

  if [[ -n "${VERSION}" ]]; then
    printf 'https://github.com/%s/releases/download/%s/%s' "${REPO}" "${VERSION}" "${file}"
  else
    printf 'https://github.com/%s/releases/latest/download/%s' "${REPO}" "${file}"
  fi
}

verify_checksum() {
  local archive_path="$1"
  local sums="$2"
  local archive_name expected actual

  archive_name="$(basename "${archive_path}")"
  grep -q "${archive_name}" "${sums}" || die "checksum entry not found for ${archive_name}"

  if have_cmd sha256sum; then
    expected="$(grep "${archive_name}" "${sums}" | awk '{print $1}')"
    actual="$(sha256sum "${archive_path}" | awk '{print $1}')"
  elif have_cmd shasum; then
    expected="$(grep "${archive_name}" "${sums}" | awk '{print $1}')"
    actual="$(shasum -a 256 "${archive_path}" | awk '{print $1}')"
  else
    log "sha256sum/shasum not found; skipping checksum verification"
    return 0
  fi

  [[ "${expected}" == "${actual}" ]] || die "checksum mismatch for ${archive_name}"
  log "checksum verified"
}

install_binary() {
  local workdir archive sums

  workdir="$(mktemp -d)"
  trap "rm -rf '${workdir}'" EXIT

  archive="${ARTIFACT}.${ARCHIVE_EXT}"
  sums="SHA256SUMS.txt"

  log "downloading ${archive}"
  download "$(asset_url "${archive}")" "${workdir}/${archive}"

  log "downloading checksums"
  if download "$(asset_url "${sums}")" "${workdir}/${sums}" 2>/dev/null; then
    verify_checksum "${workdir}/${archive}" "${workdir}/${sums}"
  else
    log "checksum file unavailable; skipping verification"
  fi

  tar -xzf "${workdir}/${archive}" -C "${workdir}"

  mkdir -p "${INSTALL_DIR}"
  install -m 0755 "${workdir}/${BINARY}" "${INSTALL_DIR}/${BINARY}"

  log "installed ${BINARY} to ${INSTALL_DIR}/${BINARY}"

  case ":${PATH}:" in
    *":${INSTALL_DIR}:"*) ;;
    *)
      log "add ${INSTALL_DIR} to your PATH, for example:"
      printf '    export PATH="%s:$PATH"\n' "${INSTALL_DIR}"
      ;;
  esac
}

main() {
  parse_args "$@"
  detect_platform
  install_binary
  log "done"
  "${INSTALL_DIR}/${BINARY}" --version 2>/dev/null || true
}

main "$@"