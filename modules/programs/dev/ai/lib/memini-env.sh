export MEMINI_BASE_URL=https://memini.plexuz.xyz
export MEMINI_REQUIRE_HTTPS=1
if ! MEMINI_API_KEY="$(infisical secrets --env default --path /Kubernetes/DexTek/Memini get MEMINI_API_KEY --plain --telemetry false)" || [ -z "$MEMINI_API_KEY" ]; then
  echo "memini: failed to fetch MEMINI_API_KEY from Infisical; memini MCP will not authenticate" >&2
  unset MEMINI_API_KEY
else
  export MEMINI_API_KEY
fi
export MEMINI_HOME="personal/krezh"
