#!/usr/bin/env bash
set -euo pipefail

export PATH="${HOME}/.local/bin:${PATH}"

if ! command -v picoclaw >/dev/null 2>&1; then
  echo "picoclaw is not installed" >&2
  exit 1
fi

config_dir="${HOME}/.config/safepaw/picoclaw"
mkdir -p "${config_dir}"

config_path="${config_dir}/${SAFEPAW_AGENT_ID}.env"
cat > "${config_path}" <<EOF
SAFEPAW_AGENT_ID=${SAFEPAW_AGENT_ID}
SAFEPAW_AGENT_NAME=${SAFEPAW_AGENT_NAME}
SAFEPAW_PROVIDER=${SAFEPAW_PROVIDER}
SAFEPAW_MODEL=${SAFEPAW_MODEL}
SAFEPAW_API_KEY_NAME=${SAFEPAW_API_KEY_NAME}
SAFEPAW_WORKSPACE_PATH=${SAFEPAW_WORKSPACE_PATH}
SAFEPAW_MAX_ITERATIONS=${SAFEPAW_MAX_ITERATIONS}
SAFEPAW_CAPABILITIES_JSON=${SAFEPAW_CAPABILITIES_JSON}
EOF

chmod 600 "${config_path}"
picoclaw --version >/dev/null 2>&1 || true

echo "==> picoclaw onboarding complete"
