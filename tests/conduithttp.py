#!/usr/bin/env python

"""
HTTP server for use in conduit tests.
"""

from optparse import OptionParser
from StringIO import StringIO
import json, signal, sys, urlparse

from mercurial import cmdutil, util

known_translations = {}

class RequestHandler(util.httpserver.BaseHTTPRequestHandler):
    def handle_request(self, param):
        from_repo = param['from_repo']
        from_scm = param['from_scm']
        to_repo = param['to_repo']
        to_scm = param['to_scm']
        revs = param['revs']

        translations = known_translations.get(
            (from_repo, from_scm, to_repo, to_scm), {})
        translated_revs = {}
        self.send_response(200)
        self.end_headers()

        f = StringIO()
        f.write("for(;;);")

        response = {}

        for rev in revs:
            if rev in translations:
                translated_revs[rev] = translations[rev]
            else:
                translated_revs[rev] = ""
        else:
            response['result'] = translated_revs
            response['error_code'] = None
            response['error_info'] = None

        f.write(json.dumps(response))
        self.wfile.write(f.getvalue())


    def do_POST(self):
        content_len = int(self.headers.getheader('content-length', 0))
        data = self.rfile.read(content_len)
        params = urlparse.parse_qs(data)
        if self.path.startswith('/intern/conduit/scmquery.get.mirrored.revs'):
            param = json.loads(params['params'][0])
            self.handle_request(param)
            return

        self.send_response(500)
        self.end_headers()


    def get_path_comps(self):
        assert self.path.startswith("/")
        return self.path[1:].split("/")


    def update(self, cmd, comps):
        (from_repo, from_scm,
         to_repo, to_scm,
         from_rev, to_rev) = comps
        key = (from_repo, from_scm, to_repo, to_scm)
        translations = known_translations.setdefault(key, {})

        if cmd == "PUT":
            translations[from_rev] = to_rev
            self.send_response(201)
            self.end_headers()
        elif cmd == "DELETE":
            translations.pop(from_rev, None)
            self.send_response(200)
            self.end_headers()

    def do_PUT(self):
        path_comps = self.get_path_comps()
        self.log_message("%s", path_comps)
        if len(path_comps) == 6:
            self.update("PUT", path_comps)
            return

        self.send_response(500)
        self.end_headers()


    def do_DELETE(self):
        path_comps = self.get_path_comps()
        if len(path_comps) == 6:
            self.update("DELETE", path_comps)
            return

        self.send_response(500)
        self.end_headers()


    def do_GET(self):
        self.send_response(200)
        self.end_headers()
        self.wfile.write(known_translations)


class simplehttpservice(object):
    def __init__(self, host, port):
        self.address = (host, port)
    def init(self):
        self.httpd = BaseHTTPServer.HTTPServer(self.address, RequestHandler)
    def run(self):
        self.httpd.serve_forever()

if __name__ == '__main__':
    parser = OptionParser()
    parser.add_option('-p', '--port', dest='port', type='int', default=8000,
        help='TCP port to listen on', metavar='PORT')
    parser.add_option('-H', '--host', dest='host', default='localhost',
        help='hostname or IP to listen on', metavar='HOST')
    parser.add_option('--pid', dest='pid',
        help='file name where the PID of the server is stored')
    parser.add_option('-f', '--foreground', dest='foreground',
        action='store_true',
        help='do not start the HTTP server in the background')
    parser.add_option('--daemon-postexec', action='append')

    (options, args) = parser.parse_args()

    signal.signal(signal.SIGTERM, lambda x, y: sys.exit(0))

    if options.foreground and options.pid:
        parser.error("options --pid and --foreground are mutually exclusive")

    opts = {'pid_file': options.pid,
            'daemon': not options.foreground,
            'daemon_postexec': options.daemon_postexec}
    service = simplehttpservice(options.host, options.port)
    cmdutil.service(opts, initfn=service.init, runfn=service.run,
                    runargs=[sys.executable, __file__] + sys.argv[1:])
