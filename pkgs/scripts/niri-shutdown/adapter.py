#!/usr/bin/env python3

import json
import os
import re
import signal
import socket
import subprocess
import sys
from pathlib import Path

NIRI = os.environ.get("NIRI_SHUTDOWN_NIRI", "niri")
CLOSE_WINDOW = re.compile(r"^/dispatch closewindow address:(\d+)$")


def run_niri(*args: str) -> str:
    result = subprocess.run(
        [NIRI, "msg", *args],
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    return result.stdout


def clients() -> str:
    windows = json.loads(run_niri("-j", "windows"))
    result = []
    for window in windows:
        client = {
            "address": str(window["id"]),
            "title": window.get("title") or "",
            "class": window.get("app_id") or "unknown",
            "xwayland": False,
        }
        if isinstance(window.get("pid"), int):
            client["pid"] = window["pid"]
        result.append(client)
    return json.dumps(result, separators=(",", ":"))


def handle(command: str) -> str:
    if command == "j/status":
        return '{"configProvider":"hyprlang"}'
    if command == "j/clients":
        return clients()
    if command == "j/layers":
        return "{}"

    close = CLOSE_WINDOW.fullmatch(command)
    if close:
        run_niri("action", "close-window", "--id", close.group(1))
        return "ok"
    if command == "/dispatch exit":
        run_niri("action", "quit", "--skip-confirmation")
        return "ok"

    raise ValueError(f"unsupported IPC command: {command}")


def serve(socket_path: Path) -> None:
    server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    server.bind(str(socket_path))
    server.listen(8)

    stopping = False

    def stop(_signum: int, _frame: object) -> None:
        nonlocal stopping
        stopping = True
        server.close()

    signal.signal(signal.SIGINT, stop)
    signal.signal(signal.SIGTERM, stop)

    while not stopping:
        try:
            connection, _ = server.accept()
        except OSError:
            if stopping:
                break
            raise

        with connection:
            try:
                command = connection.recv(8192).decode()
                response = handle(command)
            except Exception as error:
                print(f"niri-shutdown: {error}", file=sys.stderr)
                continue
            connection.sendall(response.encode())


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit("usage: adapter.py SOCKET")
    serve(Path(sys.argv[1]))
