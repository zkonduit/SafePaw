#!/usr/bin/env bash
set -euo pipefail

version="0.2.1"
release_base_url="https://github.com/sipeed/picoclaw/releases/download/v${version}"
checksum_file="picoclaw_0.2.1_checksums.txt"

arch="$(uname -m)"
case "${arch}" in
  x86_64|amd64)
    package_name="picoclaw_x86_64.deb"
    ;;
  aarch64|arm64)
    package_name="picoclaw_aarch64.deb"
    ;;
  armv7l|armv7)
    package_name="picoclaw_armv7.deb"
    ;;
  armv6l|armv6)
    package_name="picoclaw_armv6.deb"
    ;;
  loongarch64|loong64)
    package_name="picoclaw_loong64.deb"
    ;;
  mipsel|mipsle)
    package_name="picoclaw_mipsle.deb"
    ;;
  riscv64)
    package_name="picoclaw_riscv64.deb"
    ;;
  s390x)
    package_name="picoclaw_s390x.deb"
    ;;
  *)
    echo "Unsupported picoclaw architecture: ${arch}" >&2
    exit 1
    ;;
esac

tmp_dir="$(mktemp -d)"
trap 'rm -rf "${tmp_dir}"' EXIT
cd "${tmp_dir}"

curl -fsSLO "${release_base_url}/${checksum_file}"
curl -fsSLO "${release_base_url}/${package_name}"

grep "  ${package_name}\$" "${checksum_file}" | sha256sum -c -

if command -v sudo >/dev/null 2>&1; then
  sudo dpkg -i "${package_name}"
else
  dpkg -i "${package_name}"
fi

export PATH="${HOME}/.local/bin:${PATH}"
command -v picoclaw >/dev/null 2>&1

echo "==> picoclaw installation complete"
