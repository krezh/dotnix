_: {
  flake.modules.homeManager.ai =
    { pkgs, lib, ... }:
    let
      # Equivalent of `herdr integration install antigravity_cli`, reproduced declaratively.
      # Reports the Antigravity conversation id/transcript path to herdr's local socket on
      # PreInvocation so herdr can track/restore the pane; no-ops unless the session is
      # actually running inside a herdr-managed pane (HERDR_ENV/HERDR_SOCKET_PATH/HERDR_PANE_ID).
      herdrAgentStateHook = pkgs.writeShellScript "herdr-antigravity-state" ''
        set -eu
        emit_and_exit() {
          printf '{}\n'
          exit 0
        }

        [ "''${1:-}" = "session" ] || emit_and_exit
        [ "''${HERDR_ENV:-}" = "1" ] || emit_and_exit
        [ -n "''${HERDR_SOCKET_PATH:-}" ] || emit_and_exit
        [ -n "''${HERDR_PANE_ID:-}" ] || emit_and_exit

        ${lib.getExe pkgs.python3} - <<'PY' 2>/dev/null || true
        import json
        import os
        import socket
        import sys
        import time

        try:
            payload = json.load(sys.stdin)
        except Exception:
            raise SystemExit(0)

        if not isinstance(payload, dict):
            raise SystemExit(0)

        def text(name):
            value = payload.get(name)
            return value if isinstance(value, str) and value else None

        session_id = text("conversationId")
        if session_id is None:
            raise SystemExit(0)

        seq = time.time_ns()
        params = {
            "pane_id": os.environ["HERDR_PANE_ID"],
            "source": "herdr:antigravity_cli",
            "agent": "agy",
            "seq": seq,
            "agent_session_id": session_id,
        }

        transcript_path = text("transcriptPath")
        if transcript_path is not None:
            params["agent_session_path"] = transcript_path

        request = json.dumps({
            "id": f"herdr:antigravity_cli:{seq}",
            "method": "pane.report_agent_session",
            "params": params,
        })
        try:
            with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as client:
                client.settimeout(0.5)
                client.connect(os.environ["HERDR_SOCKET_PATH"])
                client.sendall((request + "\n").encode())
                client.recv(4096)
        except Exception:
            pass
        PY

        emit_and_exit
      '';
    in
    {
      home.file.".gemini/config/hooks.json".text = builtins.toJSON {
        herdr = {
          PreInvocation = [
            {
              command = "${herdrAgentStateHook} session";
              timeout = 10;
              type = "command";
            }
          ];
        };
      };
    };
}
