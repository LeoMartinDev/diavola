#!/usr/bin/env bash
# Environment bootstrap for non-interactive WSL shells.
# Strips the polluted Windows PATH (parentheses break bash) and loads fnm.
export PATH="/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:/usr/lib/wsl/lib"
export PATH="/home/leo/.cargo/bin:/home/leo/.deno/bin:$PATH"
# fnm: use the installed node version directly (avoid multishell eval issues).
export PATH="/home/leo/.local/share/fnm/node-versions/v22.14.0/installation/bin:$PATH"
export FNM_DIR="/home/leo/.local/share/fnm"

"$@"
