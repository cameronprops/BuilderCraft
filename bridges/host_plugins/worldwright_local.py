"""Bounded local Worldwright JSON-lines client for optional host plugins.

This module has no CAD-host imports or dependencies. It never discovers,
uploads, or serializes host documents. A caller explicitly chooses a command
and inputs. The existing Worldwright control server must be enabled manually.
"""
import json
import os
import socket

MAX_REQUEST_BYTES = 16 * 1024
MAX_RESPONSE_BYTES = 1024 * 1024
DEFAULT_TIMEOUT_SECONDS = 3.0


class WorldwrightConnectionError(RuntimeError):
    """A local control connection or protocol operation failed."""


def _control_port(port):
    if port is None:
        port = os.environ.get("WORLDWRIGHT_CONTROL_PORT")
    try:
        value = int(port)
    except (TypeError, ValueError) as exc:
        raise WorldwrightConnectionError(
            "Set a Worldwright local control port (1-65535)."
        ) from exc
    if not 1 <= value <= 65535:
        raise WorldwrightConnectionError("Control port must be 1-65535.")
    return value


def call_command(command, params=None, *, port=None, timeout=DEFAULT_TIMEOUT_SECONDS):
    """Execute one native Worldwright command over loopback and return result.

    A connection transports exactly one request and one response. Socket
    connections are restricted to 127.0.0.1. No automatic retries, document
    access, mutation, cloud endpoints, or background jobs are introduced here.
    """
    if not isinstance(command, str) or not command or len(command) > 128:
        raise WorldwrightConnectionError("Invalid command ID.")
    if params is None:
        params = {}
    if not isinstance(params, dict):
        raise WorldwrightConnectionError("Command params must be an object.")
    if not isinstance(timeout, (int, float)) or not 0 < timeout <= 30:
        raise WorldwrightConnectionError("Timeout must be between 0 and 30 seconds.")

    request = {
        "id": 1,
        "method": "engine.execute",
        "params": {"command": command, "params": params},
    }
    try:
        wire = json.dumps(
            request, allow_nan=False, ensure_ascii=True, separators=(",", ":")
        ).encode("utf-8") + b"\n"
    except (TypeError, ValueError) as exc:
        raise WorldwrightConnectionError("Command arguments are not finite JSON.") from exc
    if len(wire) > MAX_REQUEST_BYTES:
        raise WorldwrightConnectionError("Command exceeds local request budget.")

    try:
        with socket.create_connection(("127.0.0.1", _control_port(port)), timeout=timeout) as sock:
            sock.settimeout(timeout)
            sock.sendall(wire)
            with sock.makefile("rb") as stream:
                reply_line = stream.readline(MAX_RESPONSE_BYTES + 1)
    except (OSError, TimeoutError) as exc:
        raise WorldwrightConnectionError(
            "Worldwright local control is unavailable or timed out."
        ) from exc

    if not reply_line or len(reply_line) > MAX_RESPONSE_BYTES or not reply_line.endswith(b"\n"):
        raise WorldwrightConnectionError("Incomplete or oversized Worldwright response.")
    try:
        reply = json.loads(reply_line)
    except (UnicodeError, json.JSONDecodeError) as exc:
        raise WorldwrightConnectionError("Malformed Worldwright response.") from exc
    if not isinstance(reply, dict) or type(reply.get("id")) is not int or reply["id"] != 1:
        raise WorldwrightConnectionError("Worldwright response ID mismatch.")
    if type(reply.get("ok")) is not bool:
        raise WorldwrightConnectionError("Worldwright response missing success status.")
    if not reply["ok"]:
        message = reply.get("error")
        if not isinstance(message, str) or not message:
            message = "Unknown command failure."
        raise WorldwrightConnectionError("Worldwright command failed: " + message[:1024])
    if "result" not in reply:
        raise WorldwrightConnectionError("Worldwright response has no result.")
    return reply["result"]
