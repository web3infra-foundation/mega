from http.server import HTTPServer, BaseHTTPRequestHandler
import json

class Handler(BaseHTTPRequestHandler):
    def do_POST(self):
        length = int(self.headers.get('Content-Length', 0))
        body = self.rfile.read(length)
        print(f'\n--- Webhook Received ---')
        print(f'X-Mega-Event: {self.headers.get("X-Mega-Event")}')
        print(f'X-Mega-Signature: {self.headers.get("X-Mega-Signature")}')
        print(json.dumps(json.loads(body), indent=2))
        self.send_response(200)
        self.end_headers()

HTTPServer(('0.0.0.0', 9001), Handler).serve_forever()
