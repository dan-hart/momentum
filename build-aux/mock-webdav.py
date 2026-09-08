#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Minimal WebDAV mock: GET/PUT with ETag, If-Match, If-None-Match, MKCOL, 404s.
import http.server, hashlib, sys
files = {}
class H(http.server.BaseHTTPRequestHandler):
    def log_message(self, *a): pass
    def etag(self, body): return '"%s"' % hashlib.md5(body).hexdigest()
    def do_GET(self):
        if self.path not in files: self.send_response(404); self.end_headers(); return
        b = files[self.path]; self.send_response(200); self.send_header("OC-ETag", self.etag(b)); self.send_header("Content-Length", str(len(b))); self.end_headers(); self.wfile.write(b)
    def do_MKCOL(self): self.send_response(201); self.end_headers()
    def do_PUT(self):
        body = self.rfile.read(int(self.headers.get("Content-Length", 0)))
        cur = files.get(self.path)
        if "If-Match" in self.headers and (cur is None or self.headers["If-Match"] != self.etag(cur)): self.send_response(412); self.end_headers(); return
        if self.headers.get("If-None-Match") == "*" and cur is not None: self.send_response(412); self.end_headers(); return
        files[self.path] = body; self.send_response(201 if cur is None else 204); self.send_header("OC-ETag", self.etag(body)); self.end_headers()
http.server.ThreadingHTTPServer(("127.0.0.1", int(sys.argv[1])), H).serve_forever()
