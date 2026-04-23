#!/usr/bin/env python3
"""Local fixture for validating Sentinel SQL injection traffic probes.

Usage:
  python3 scripts/sqli_probe_fixture.py --host 127.0.0.1 --port 18080

Endpoints:
  GET  /
  GET  /health
  GET  /search?q=agent
  POST /search           {"q":"agent"}

Behavior:
  - Stable 200 baseline for normal input.
  - Explicit SQL error disclosure for quote-breaking probes.
  - Boolean blind behavior change for true/false predicates.
  - Time blind delay for SLEEP/pg_sleep/WAITFOR probes.
  - Optional SQL leakage in response headers.
"""

from __future__ import annotations

import argparse
import json
import re
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import parse_qs, urlparse


MYSQL_ERROR_TEXT = (
    "You have an error in your SQL syntax; check the manual that corresponds "
    "to your MySQL server version for the right syntax to use near '''"
)

SQL_HEADER_ERROR_TEXT = "com.microsoft.sqlserver.jdbc.SQLServerException: Incorrect syntax near '''"

TIME_DELAY_SECONDS = 4.2

TRUE_BOOLEAN_PATTERNS = [
    re.compile(r"\b(?:and|or)\s+1=1\b", re.IGNORECASE),
    re.compile(r"\b(?:and|or)\s+'1'='1'\b", re.IGNORECASE),
]

FALSE_BOOLEAN_PATTERNS = [
    re.compile(r"\b(?:and|or)\s+1=2\b", re.IGNORECASE),
    re.compile(r"\b(?:and|or)\s+'1'='2'\b", re.IGNORECASE),
]

TIME_PATTERNS = [
    re.compile(r"\bsleep\s*\(\s*4\s*\)", re.IGNORECASE),
    re.compile(r"\bpg_sleep\s*\(\s*4\s*\)", re.IGNORECASE),
    re.compile(r"waitfor\s+delay\s+'0:0:4'", re.IGNORECASE),
]


def build_ok_payload(query: str) -> dict:
    return {
        "ok": True,
        "query": query,
        "items": [
            {
                "id": "doc-1001",
                "title": "sentinel agent collab",
                "score": 98,
            }
        ],
        "count": 1,
    }


def build_empty_payload(query: str) -> dict:
    return {
        "ok": True,
        "query": query,
        "items": [],
        "count": 0,
    }


def build_sql_error_payload(query: str) -> dict:
    return {
        "ok": False,
        "error": MYSQL_ERROR_TEXT,
        "query": query,
    }


def detect_query_mode(query: str) -> str:
    if any(pattern.search(query) for pattern in TIME_PATTERNS):
        return "time"
    if any(pattern.search(query) for pattern in FALSE_BOOLEAN_PATTERNS):
        return "boolean_false"
    if any(pattern.search(query) for pattern in TRUE_BOOLEAN_PATTERNS):
        return "boolean_true"
    if "'" in query or '"' in query:
        return "error"
    return "normal"


def build_homepage(port: int) -> bytes:
    html = f"""<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <title>Sentinel SQLi Fixture</title>
    <style>
      :root {{
        color-scheme: light;
        font-family: "Iowan Old Style", "Georgia", serif;
        background: #f4efe4;
        color: #1d1a15;
      }}
      body {{
        margin: 0;
        background:
          radial-gradient(circle at top right, rgba(168, 120, 58, 0.16), transparent 32rem),
          linear-gradient(180deg, #f8f4ec 0%, #efe5d2 100%);
      }}
      main {{
        max-width: 56rem;
        margin: 0 auto;
        padding: 3rem 1.5rem 4rem;
      }}
      h1 {{
        margin-bottom: 0.4rem;
        font-size: 2.6rem;
      }}
      p {{
        line-height: 1.6;
      }}
      .panel {{
        margin-top: 1.5rem;
        padding: 1.25rem;
        border: 1px solid rgba(61, 46, 24, 0.18);
        border-radius: 1rem;
        background: rgba(255, 251, 244, 0.82);
        box-shadow: 0 0.8rem 2rem rgba(53, 38, 15, 0.08);
      }}
      code {{
        padding: 0.1rem 0.35rem;
        border-radius: 0.3rem;
        background: rgba(61, 46, 24, 0.08);
      }}
      input, button {{
        font: inherit;
      }}
      input {{
        width: min(100%, 28rem);
        padding: 0.7rem 0.9rem;
        border: 1px solid rgba(61, 46, 24, 0.22);
        border-radius: 0.7rem;
        background: #fffdfa;
      }}
      button {{
        margin-left: 0.4rem;
        padding: 0.7rem 1rem;
        border: 0;
        border-radius: 999px;
        background: #7a4d18;
        color: #fffaf2;
        cursor: pointer;
      }}
      ul {{
        line-height: 1.8;
      }}
      .muted {{
        color: #5a4b34;
      }}
    </style>
  </head>
  <body>
    <main>
      <h1>Sentinel SQL Injection Fixture</h1>
      <p class="muted">
        Use this page to generate stable traffic for the SQL injection detector.
        Start with a normal request, then let Sentinel replay probes automatically.
      </p>

      <section class="panel">
        <h2>GET baseline</h2>
        <form method="get" action="/search">
          <input name="q" value="agent" />
          <button type="submit">Send GET /search</button>
        </form>
      </section>

      <section class="panel">
        <h2>POST baseline</h2>
        <p>Use your browser devtools, repeater, or curl:</p>
        <p><code>curl -i -X POST http://127.0.0.1:{port}/search -H 'Content-Type: application/json' -d '{{"q":"agent"}}'</code></p>
      </section>

      <section class="panel">
        <h2>Probe reference</h2>
        <ul>
          <li><code>agent'</code> or <code>agent"</code>: explicit SQL error disclosure</li>
          <li><code>agent AND 1=1</code>: boolean true branch</li>
          <li><code>agent AND 1=2</code>: boolean false branch</li>
          <li><code>agent' AND SLEEP(4)--</code>: time blind delay</li>
          <li><code>agent' header_leak</code>: SQL error echoed in response headers</li>
        </ul>
      </section>
    </main>
  </body>
</html>
"""
    return html.encode("utf-8")


class FixtureHandler(BaseHTTPRequestHandler):
    server_version = "SentinelSQLiFixture/2.0"

    def log_message(self, fmt: str, *args) -> None:
        print(
            f"{self.address_string()} - - [{self.log_date_time_string()}] {fmt % args}",
            flush=True,
        )

    def do_GET(self) -> None:
        parsed = urlparse(self.path)
        if parsed.path == "/":
            self._write_html(200, build_homepage(self.server.server_port))
            return

        if parsed.path == "/health":
            self._write_json(200, {"ok": True, "service": "sqli-probe-fixture"})
            return

        if parsed.path != "/search":
            self._write_json(404, {"ok": False, "error": "not_found"})
            return

        query = parse_qs(parsed.query).get("q", [""])[0]
        self._handle_search(query)

    def do_POST(self) -> None:
        parsed = urlparse(self.path)
        if parsed.path != "/search":
            self._write_json(404, {"ok": False, "error": "not_found"})
            return

        content_length = int(self.headers.get("Content-Length", "0"))
        raw_body = self.rfile.read(content_length) if content_length > 0 else b""

        try:
            body = json.loads(raw_body.decode("utf-8") or "{}")
        except json.JSONDecodeError:
            self._write_json(400, {"ok": False, "error": "invalid_json"})
            return

        query = body.get("q", "")
        if not isinstance(query, str):
            self._write_json(400, {"ok": False, "error": "q_must_be_string"})
            return

        self._handle_search(query)

    def _handle_search(self, query: str) -> None:
        mode = detect_query_mode(query)
        extra_headers = {}

        if mode == "time":
            time.sleep(TIME_DELAY_SECONDS)
            self._write_json(200, build_ok_payload(query), extra_headers=extra_headers)
            return

        if "header_leak" in query.lower():
            extra_headers["X-Exception-Type"] = SQL_HEADER_ERROR_TEXT

        if mode == "boolean_false":
            self._write_json(200, build_empty_payload(query), extra_headers=extra_headers)
            return

        if mode == "boolean_true":
            self._write_json(200, build_ok_payload(query), extra_headers=extra_headers)
            return

        if mode == "error":
            self._write_json(500, build_sql_error_payload(query), extra_headers=extra_headers)
            return

        self._write_json(200, build_ok_payload(query), extra_headers=extra_headers)

    def _write_json(self, status: int, payload: dict, extra_headers: dict[str, str] | None = None) -> None:
        body = json.dumps(payload, ensure_ascii=True, separators=(",", ":")).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Cache-Control", "no-store")
        for name, value in (extra_headers or {}).items():
            self.send_header(name, value)
        self.end_headers()
        self.wfile.write(body)

    def _write_html(self, status: int, body: bytes) -> None:
        self.send_response(status)
        self.send_header("Content-Type", "text/html; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        self.wfile.write(body)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Run a local SQLi probe fixture server.")
    parser.add_argument("--host", default="127.0.0.1", help="bind host")
    parser.add_argument("--port", default=18080, type=int, help="bind port")
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    server = ThreadingHTTPServer((args.host, args.port), FixtureHandler)
    print(
        f"Sentinel SQLi probe fixture listening on http://{args.host}:{args.port}",
        flush=True,
    )
    print("Open / for guided manual testing.", flush=True)
    print(f"Error-based example:  http://{args.host}:{args.port}/search?q=agent", flush=True)
    print(f"Time-based example:   http://{args.host}:{args.port}/search?q=agent%27%20AND%20SLEEP(4)--", flush=True)
    server.serve_forever()


if __name__ == "__main__":
    main()
