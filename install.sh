#!/usr/bin/env bash
set -euo pipefail

SAFEPAW_RELEASE_OWNER="${SAFEPAW_RELEASE_OWNER:-zkonduit}"
SAFEPAW_RELEASE_REPO="${SAFEPAW_RELEASE_REPO:-SafePaw}"
SAFEPAW_VERSION="${SAFEPAW_VERSION:-latest}"
SAFEPAW_INSTALL_DIR="${SAFEPAW_INSTALL_DIR:-}"
SAFEPAW_SKIP_MULTIPASS="${SAFEPAW_SKIP_MULTIPASS:-0}"
SAFEPAW_OVERWRITE="${SAFEPAW_OVERWRITE:-0}"

MULTIPASS_INSTALL_DOCS="https://documentation.ubuntu.com/multipass/stable/how-to-guides/install-multipass/"
SAFEPAW_RELEASES_BASE_URL="https://github.com/${SAFEPAW_RELEASE_OWNER}/${SAFEPAW_RELEASE_REPO}/releases"

log() {
  printf '==> %s\n' "$*"
}

warn() {
  printf 'warning: %s\n' "$*" >&2
}

die() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

have_command() {
  command -v "$1" >/dev/null 2>&1
}

print_usage() {
  cat <<'EOF'
Usage: install.sh [options]

Options:
  --overwrite, --force     Replace an existing safepaw binary at the target path.
  --install-dir DIR        Install into DIR instead of the default location.
  --version VERSION        Install a specific release tag instead of latest.
  --skip-multipass         Skip automatic Multipass installation.
  -h, --help               Show this help text.

Environment variables:
  SAFEPAW_INSTALL_DIR
  SAFEPAW_OVERWRITE=1
  SAFEPAW_SKIP_MULTIPASS=1
  SAFEPAW_VERSION=vX.Y.Z

Examples:
  curl -fsSL https://raw.githubusercontent.com/zkonduit/SafePaw/main/install.sh | bash
  curl -fsSL https://raw.githubusercontent.com/zkonduit/SafePaw/main/install.sh | bash -s -- --overwrite
  curl -fsSL https://raw.githubusercontent.com/zkonduit/SafePaw/main/install.sh | \
    SAFEPAW_VERSION=v0.1.0 bash -s -- --install-dir "$HOME/.local/bin"
EOF
}

parse_args() {
  while [ "$#" -gt 0 ]; do
    case "$1" in
      --overwrite|--force)
        SAFEPAW_OVERWRITE=1
        ;;
      --install-dir)
        shift
        [ "$#" -gt 0 ] || die "--install-dir requires a directory path."
        SAFEPAW_INSTALL_DIR="$1"
        ;;
      --version)
        shift
        [ "$#" -gt 0 ] || die "--version requires a release tag."
        SAFEPAW_VERSION="$1"
        ;;
      --skip-multipass)
        SAFEPAW_SKIP_MULTIPASS=1
        ;;
      -h|--help)
        print_usage
        exit 0
        ;;
      --)
        shift
        break
        ;;
      *)
        die "Unknown argument: $1. See --help for supported options."
        ;;
    esac
    shift
  done

  [ "$#" -eq 0 ] || die "Unexpected positional argument: $1"
}

path_contains() {
  case ":${PATH}:" in
    *":$1:"*) return 0 ;;
    *) return 1 ;;
  esac
}

have_tty() {
  [ -r /dev/tty ] && [ -w /dev/tty ]
}

run_as_root() {
  if [ "$(id -u)" -eq 0 ]; then
    "$@"
  elif have_command sudo; then
    sudo "$@"
  else
    die "This step requires sudo: $*"
  fi
}

download_file() {
  url="$1"
  output_path="$2"

  if have_command curl; then
    curl -fsSL "$url" -o "$output_path"
    return
  fi

  if have_command wget; then
    wget -qO "$output_path" "$url"
    return
  fi

  die "curl or wget is required to download release assets."
}

is_wsl() {
  [ -n "${WSL_DISTRO_NAME:-}" ] || grep -qi microsoft /proc/sys/kernel/osrelease 2>/dev/null
}

find_multipass_bin() {
  if have_command multipass; then
    command -v multipass
    return 0
  fi

  if [ -x /snap/bin/multipass ]; then
    printf '%s\n' /snap/bin/multipass
    return 0
  fi

  return 1
}

ensure_snap_path() {
  if [ -d /snap/bin ] && ! path_contains /snap/bin; then
    PATH="/snap/bin:${PATH}"
    export PATH
  fi
}

detect_platform() {
  os_name="$(uname -s)"

  case "$os_name" in
    Darwin)
      PLATFORM="macos"
      ;;
    Linux)
      if is_wsl; then
        die "WSL is not supported for automatic Multipass setup. Install Multipass on the host first: ${MULTIPASS_INSTALL_DOCS}"
      fi
      PLATFORM="linux"
      ;;
    MINGW*|MSYS*|CYGWIN*)
      PLATFORM="windows"
      ;;
    *)
      die "Unsupported operating system: ${os_name}"
      ;;
  esac
}

detect_arch() {
  arch_name="$(uname -m)"

  case "$arch_name" in
    x86_64|amd64)
      ARCH="x86_64"
      ;;
    arm64|aarch64)
      ARCH="aarch64"
      ;;
    *)
      die "Unsupported architecture: ${arch_name}"
      ;;
  esac
}

ensure_linux_libc() {
  if [ "$PLATFORM" != "linux" ]; then
    return
  fi

  if have_command ldd && ldd --version 2>&1 | grep -qi musl; then
    die "musl-based Linux distributions such as Alpine are not supported by the published SafePaw binaries."
  fi
}

install_multipass_macos() {
  if ! have_command brew; then
    die "Homebrew is required for automatic Multipass installation on macOS. Install Multipass manually: ${MULTIPASS_INSTALL_DOCS}"
  fi

  log "Installing Multipass via Homebrew"
  brew install --cask multipass
}

install_multipass_linux() {
  ensure_snap_path

  if have_command snap; then
    log "Installing Multipass via snap"
    run_as_root snap install multipass
    ensure_snap_path
    return
  fi

  distro_id="linux"
  if [ -f /etc/os-release ]; then
    distro_id="$(
      . /etc/os-release
      printf '%s' "${ID:-linux}"
    )"
  fi

  case "$distro_id" in
    ubuntu|debian|linuxmint|pop|elementary|neon|kali)
      if ! have_command apt-get; then
        die "apt-get is not available, so snapd cannot be installed automatically. Install Multipass manually: ${MULTIPASS_INSTALL_DOCS}"
      fi

      log "Installing snapd via apt-get"
      run_as_root apt-get update
      run_as_root apt-get install -y snapd

      if have_command systemctl; then
        run_as_root systemctl enable --now snapd.socket
      fi

      if [ ! -e /snap ] && [ -d /var/lib/snapd/snap ]; then
        run_as_root ln -s /var/lib/snapd/snap /snap || true
      fi

      ensure_snap_path
      log "Installing Multipass via snap"
      run_as_root snap install multipass
      ;;
    fedora|rhel|centos|rocky|almalinux)
      if ! have_command dnf; then
        die "dnf is not available, so snapd cannot be installed automatically. Install Multipass manually: ${MULTIPASS_INSTALL_DOCS}"
      fi

      if ! have_command systemctl; then
        die "systemctl is required to enable snapd on this Linux host. Install Multipass manually: ${MULTIPASS_INSTALL_DOCS}"
      fi

      log "Installing snapd via dnf"
      run_as_root dnf install -y snapd
      run_as_root systemctl enable --now snapd.socket

      if [ ! -e /snap ] && [ -d /var/lib/snapd/snap ]; then
        run_as_root ln -s /var/lib/snapd/snap /snap || true
      fi

      ensure_snap_path
      log "Installing Multipass via snap"
      run_as_root snap install multipass
      ;;
    *)
      die "Automatic Multipass installation is only implemented for snap-based Linux hosts right now. Install Multipass manually: ${MULTIPASS_INSTALL_DOCS}"
      ;;
  esac
}

ensure_multipass() {
  if [ "$SAFEPAW_SKIP_MULTIPASS" = "1" ]; then
    warn "Skipping Multipass installation because SAFEPAW_SKIP_MULTIPASS=1."
    return
  fi

  if multipass_bin="$(find_multipass_bin)"; then
    log "Found Multipass at ${multipass_bin}"
    return
  fi

  case "$PLATFORM" in
    macos)
      install_multipass_macos
      ;;
    linux)
      install_multipass_linux
      ;;
    windows)
      die "Automatic Multipass installation is not supported from bash on Windows. Install it first: ${MULTIPASS_INSTALL_DOCS}"
      ;;
    *)
      die "Unsupported platform for Multipass installation: ${PLATFORM}"
      ;;
  esac

  if ! multipass_bin="$(find_multipass_bin)"; then
    die "Multipass installation finished, but the multipass command is still unavailable. See ${MULTIPASS_INSTALL_DOCS}"
  fi

  if ! "$multipass_bin" version >/dev/null 2>&1; then
    warn "Multipass is installed, but the daemon may still be starting."
  fi
}

resolve_install_dir() {
  if [ -n "$SAFEPAW_INSTALL_DIR" ]; then
    printf '%s\n' "$SAFEPAW_INSTALL_DIR"
    return
  fi

  if [ "$(id -u)" -eq 0 ] || [ -w /usr/local/bin ]; then
    printf '%s\n' /usr/local/bin
    return
  fi

  printf '%s/.local/bin\n' "$HOME"
}

resolve_shell_profile() {
  shell_name="$(basename "${SHELL:-sh}")"

  case "$shell_name" in
    zsh)
      PROFILE_SHELL="zsh"
      PROFILE_PATH="${HOME}/.zshrc"
      ;;
    bash)
      PROFILE_SHELL="bash"
      if [ -f "${HOME}/.bashrc" ]; then
        PROFILE_PATH="${HOME}/.bashrc"
      elif [ -f "${HOME}/.bash_profile" ]; then
        PROFILE_PATH="${HOME}/.bash_profile"
      else
        PROFILE_PATH="${HOME}/.bashrc"
      fi
      ;;
    fish)
      PROFILE_SHELL="fish"
      PROFILE_PATH="${HOME}/.config/fish/config.fish"
      ;;
    *)
      PROFILE_SHELL="sh"
      PROFILE_PATH="${HOME}/.profile"
      ;;
  esac
}

build_persistent_path_line() {
  install_dir="$1"
  resolve_shell_profile

  case "$PROFILE_SHELL" in
    fish)
      printf 'fish_add_path -g -- "%s"\n' "$install_dir"
      ;;
    *)
      printf 'export PATH="%s:$PATH"\n' "$install_dir"
      ;;
  esac
}

build_current_shell_path_command() {
  install_dir="$1"
  resolve_shell_profile

  case "$PROFILE_SHELL" in
    fish)
      printf 'fish_add_path -- "%s"' "$install_dir"
      ;;
    *)
      printf 'export PATH="%s:$PATH"' "$install_dir"
      ;;
  esac
}

build_reload_command() {
  resolve_shell_profile

  case "$PROFILE_SHELL" in
    fish)
      printf 'source %s' "$PROFILE_PATH"
      ;;
    *)
      printf '. %s' "$PROFILE_PATH"
      ;;
  esac
}

maybe_persist_path() {
  install_dir="$1"

  case "$install_dir" in
    "$HOME"|"$HOME"/*) ;;
    *) return 1 ;;
  esac

  resolve_shell_profile
  profile_dir="$(dirname "$PROFILE_PATH")"
  path_line="$(build_persistent_path_line "$install_dir")"
  home_relative_install_dir=""

  case "$install_dir" in
    "$HOME")
      home_relative_install_dir="\$HOME"
      ;;
    "$HOME"/*)
      home_relative_install_dir="\$HOME/${install_dir#"$HOME"/}"
      ;;
  esac

  if ! mkdir -p "$profile_dir" 2>/dev/null; then
    return 1
  fi

  if [ -f "$PROFILE_PATH" ]; then
    if grep -F "$install_dir" "$PROFILE_PATH" >/dev/null 2>&1; then
      PATH_PERSIST_STATUS="already_present"
      return 0
    fi

    if [ -n "$home_relative_install_dir" ] && grep -F "$home_relative_install_dir" "$PROFILE_PATH" >/dev/null 2>&1; then
      PATH_PERSIST_STATUS="already_present"
      return 0
    fi
  fi

  if ! printf '\n%s\n' "$path_line" >>"$PROFILE_PATH" 2>/dev/null; then
    return 1
  fi

  PATH_PERSIST_STATUS="added"
  return 0
}

print_path_instructions() {
  install_dir="$1"
  resolve_shell_profile
  current_shell_command="$(build_current_shell_path_command "$install_dir")"
  reload_command="$(build_reload_command)"
  path_line="$(build_persistent_path_line "$install_dir")"

  warn "${install_dir} is not currently on PATH."
  log "Run this now to use safepaw in the current shell:"
  printf '    %s\n' "$current_shell_command"
  log "Persist it by adding this line to ${PROFILE_PATH}:"
  printf '    %s\n' "$path_line"
  log "Then reload your shell:"
  printf '    %s\n' "$reload_command"
}

ensure_directory() {
  dir_path="$1"

  if [ -d "$dir_path" ]; then
    return
  fi

  if mkdir -p "$dir_path" 2>/dev/null; then
    return
  fi

  run_as_root mkdir -p "$dir_path"
}

confirm_overwrite() {
  destination_path="$1"

  if [ -d "$destination_path" ]; then
    die "Cannot install SafePaw to ${destination_path} because it is a directory."
  fi

  if [ ! -e "$destination_path" ]; then
    return
  fi

  if [ "$SAFEPAW_OVERWRITE" = "1" ]; then
    warn "Overwriting existing file at ${destination_path} because --overwrite was supplied."
    return
  fi

  warn "An existing safepaw binary was found at ${destination_path}."

  if ! have_tty; then
    die "Refusing to overwrite ${destination_path} without confirmation. Re-run with --overwrite to replace it."
  fi

  printf 'Overwrite it? [y/N] ' >/dev/tty
  read -r overwrite_reply </dev/tty || die "Could not read overwrite confirmation from /dev/tty."

  case "$overwrite_reply" in
    y|Y|yes|YES)
      ;;
    *)
      die "Installation aborted. Existing file left unchanged at ${destination_path}."
      ;;
  esac
}

install_binary() {
  source_path="$1"
  destination_path="$2"
  destination_dir="$(dirname "$destination_path")"

  ensure_directory "$destination_dir"

  if install -m 0755 "$source_path" "$destination_path" 2>/dev/null; then
    return
  fi

  run_as_root install -m 0755 "$source_path" "$destination_path"
}

normalized_version_tag() {
  if [ "$SAFEPAW_VERSION" = "latest" ]; then
    printf '%s\n' latest
    return
  fi

  printf 'v%s\n' "${SAFEPAW_VERSION#v}"
}

build_asset_name() {
  case "$PLATFORM" in
    linux)
      printf 'safepaw-linux-%s\n' "$ARCH"
      ;;
    macos)
      printf 'safepaw-macos-%s\n' "$ARCH"
      ;;
    windows)
      if [ "$ARCH" != "x86_64" ]; then
        die "Windows ARM64 releases are not published right now."
      fi
      printf 'safepaw-windows-x86_64.exe\n'
      ;;
    *)
      die "Unsupported platform for SafePaw asset selection: ${PLATFORM}"
      ;;
  esac
}

build_asset_url() {
  asset_name="$1"
  version_tag="$(normalized_version_tag)"

  if [ "$version_tag" = "latest" ]; then
    printf '%s/latest/download/%s\n' "$SAFEPAW_RELEASES_BASE_URL" "$asset_name"
    return
  fi

  printf '%s/download/%s/%s\n' "$SAFEPAW_RELEASES_BASE_URL" "$version_tag" "$asset_name"
}

main() {
  parse_args "$@"
  detect_platform
  detect_arch
  ensure_linux_libc
  ensure_multipass

  install_dir="$(resolve_install_dir)"
  binary_name="safepaw"
  asset_name="$(build_asset_name)"
  asset_url="$(build_asset_url "$asset_name")"

  if [ "$PLATFORM" = "windows" ]; then
    binary_name="safepaw.exe"
  fi

  temp_dir="$(mktemp -d)"
  trap 'rm -rf "$temp_dir"' EXIT

  release_path="${temp_dir}/${asset_name}"
  destination_path="${install_dir}/${binary_name}"

  log "Downloading ${asset_name} from ${asset_url}"
  if ! download_file "$asset_url" "$release_path"; then
    die "Failed to download ${asset_name}. Check that version ${SAFEPAW_VERSION} exists for ${PLATFORM}/${ARCH}."
  fi

  confirm_overwrite "$destination_path"
  install_binary "$release_path" "$destination_path"

  if ! "$destination_path" --help >/dev/null 2>&1; then
    warn "Installed binary did not pass the --help smoke test."
  fi

  log "Installed SafePaw to ${destination_path}"
  if ! path_contains "$install_dir"; then
    if maybe_persist_path "$install_dir"; then
      reload_command="$(build_reload_command)"
      if [ "$PATH_PERSIST_STATUS" = "added" ]; then
        log "Added ${install_dir} to PATH in ${PROFILE_PATH}."
      else
        log "${PROFILE_PATH} already contains a PATH entry for ${install_dir}."
      fi
      warn "Your current shell still needs to be reloaded before 'safepaw' will resolve by name."
      log "Reload your shell with: ${reload_command}"
      log "Next step: ${reload_command} && ${binary_name} start"
      return
    fi

    print_path_instructions "$install_dir"
    log "Next step: $(build_current_shell_path_command "$install_dir") && ${binary_name} start"
    return
  fi

  log "Next step: ${binary_name} start"
}

main "$@"
