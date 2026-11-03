#!/usr/bin/env python3
"""Smoke-test the built Rust MCP JSON-RPC server without desktop permissions.

Requires only Python's standard library. No mouse/keyboard events are delivered,
and no screen content is requested. Uses the stdio transport like the TS plugin.
"""
from __future__ import annotations

import json
import os
import pathlib
import queue
import subprocess
import sys
import threading
from typing import Any


def main() -> None:
    if len(sys.argv) != 2:
        raise SystemExit("usage: python smoke_mcp.py PATH_TO_RUST_EXE")
    command = pathlib.Path(sys.argv[1]).resolve()
    if not command.is_file():
        raise FileNotFoundError(command)

    env = dict(os.environ, CUA_ALLOW_CAPTURE="0", CUA_ALLOW_INPUT="0")
    proc = subprocess.Popen(
        [str(command)],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        text=True,
        encoding="utf-8",
        errors="strict",
        bufsize=1,
        env=env,
    )
    assert proc.stdin is not None and proc.stdout is not None
    messages: queue.Queue[dict[str, Any]] = queue.Queue()

    def read_output() -> None:
        for line in proc.stdout:
            try:
                messages.put(json.loads(line))
            except json.JSONDecodeError as exc:
                raise AssertionError(f"non-JSON stdout line: {line!r}") from exc

    reader = threading.Thread(target=read_output, daemon=True)
    reader.start()

    def request(request_id: int, method: str, params: dict[str, Any]) -> dict[str, Any]:
        assert proc.poll() is None, f"server died before {method}"
        wire = {"jsonrpc": "2.0", "id": request_id, "method": method, "params": params}
        proc.stdin.write(json.dumps(wire) + "\n")
        proc.stdin.flush()
        while True:
            try:
                msg = messages.get(timeout=15)
            except queue.Empty:
                raise AssertionError(f"server did not answer {method} within 15 s")
            if msg.get("id") == request_id:
                assert "error" not in msg, f"{method} protocol error: {msg}"
                return msg["result"]

    try:
        result = request(
            1,
            "initialize",
            {
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": {"name": "dsh-native-cua-ci", "version": "0.1.0"},
            },
        )
        assert result.get("serverInfo", {}).get("name") == "dsh-native-cua", result
        proc.stdin.write('{"jsonrpc":"2.0","method":"notifications/initialized"}\n')
        proc.stdin.flush()

        catalog = request(2, "tools/list", {})
        names = {entry["name"] for entry in catalog["tools"]}
        expected = {"health", "list_displays", "list_windows", "observe_ui", "screenshot", "act"}
        assert names == expected, f"tool list differs: {names}"

        health = request(3, "tools/call", {"name": "health", "arguments": {}})
        text = health["content"][0]["text"]
        flags = json.loads(text)
        assert flags == {
            "ready": True,
            "capture_enabled": False,
            "input_enabled": False,
        }, flags

        denied_input = request(
            4,
            "tools/call",
            {"name": "act", "arguments": {"actions": [{"kind": "key", "key": "enter"}]}},
        )
        assert denied_input.get("isError") is True, denied_input
        assert "disabled" in denied_input["content"][0]["text"].lower(), denied_input

        denied_capture = request(
            5,
            "tools/call",
            {"name": "observe_ui", "arguments": {"format": "compact"}},
        )
        assert denied_capture.get("isError") is True, denied_capture
        assert "disabled" in denied_capture["content"][0]["text"].lower(), denied_capture
        print("MCP handshake, tool catalog, health, and denial gates: PASS")
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=4)
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.wait(timeout=4)


if __name__ == "__main__":
    main()
