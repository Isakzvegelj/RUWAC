"""Bounded native client adapter; daemon remains the sole VT/PTY owner."""
import json, os, queue, socket, stat, struct, threading, time, uuid
from pathlib import Path
MAX_FRAME = 1048576
class Rpc:
    def __init__(self, path):
        p = Path(path)
        for item, mode in ((p.parent, 0o700), (p, 0o600)):
            info = item.stat()
            if info.st_uid != os.geteuid() or stat.S_IMODE(info.st_mode) != mode:
                raise PermissionError('Expected private owner-only socket and directory')
        if not stat.S_ISSOCK(p.stat().st_mode):
            raise PermissionError('Expected a Unix socket')
        self.socket = socket.socket(socket.AF_UNIX)
        self.socket.settimeout(2)
        self.socket.connect(str(p))
        # Linux authenticates the server, independently of pathname ownership.
        if hasattr(socket, 'SO_PEERCRED'):
            _, uid, _ = struct.unpack('3i', self.socket.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
            if uid != os.geteuid():
                self.close()
                raise PermissionError('Server identity differs from client owner')
    def read(self, n):
        result = bytearray()
        while len(result) < n:
            data = self.socket.recv(n-len(result))
            if not data: raise ConnectionError('Daemon disconnected')
            result.extend(data)
        return bytes(result)
    def call(self, method, params=None):
        rid = uuid.uuid4().hex
        body = json.dumps({'version':0,'id':rid,'method':method,'params':params or {}}, ensure_ascii=False).encode()
        if len(body)>MAX_FRAME: raise ValueError('Request exceeds frame limit')
        self.socket.sendall(struct.pack('!I',len(body))+body)
        n = struct.unpack('!I',self.read(4))[0]
        if not 0<n<=MAX_FRAME: raise ValueError('Invalid response frame size')
        response = json.loads(self.read(n))
        if response.get('id') != rid: raise ValueError('Response correlation differs')
        if 'error' in response: raise RuntimeError(response['error']['code']+': '+response['error']['message'])
        return response['result']
    def close(self): self.socket.close()
class Worker:
    def __init__(self, path, session, deliver):
        self.path, self.session, self.deliver = path, session, deliver
        self.commands = queue.Queue(maxsize=128)
        self.stop = threading.Event()
        self.thread = threading.Thread(target=self.run, daemon=True)
        self.thread.start()
    def send(self, method, params):
        if self.stop.is_set(): raise RuntimeError('View disconnected')
        try: self.commands.put_nowait((method,params))
        except queue.Full: raise BufferError('Input queue is full; input was not sent')
    def close(self): self.stop.set()
    def run(self):
        rpc = None
        try:
            rpc = Rpc(self.path)
            rpc.call('session.attach',{'session':self.session,'rows':24,'cols':80})
            revision = None
            while not self.stop.is_set():
                # Bounded burst prevents input starvation of rendering.
                for _ in range(16):
                    try: method, params = self.commands.get_nowait()
                    except queue.Empty: break
                    rpc.call(method,params)
                frame = rpc.call('client.render',{} if revision is None else {'revision':revision})
                if not frame.get('unchanged'):
                    revision = frame['revision']; self.deliver(frame,None)
                self.stop.wait(.04)
            rpc.call('session.detach')
        except Exception as error:
            self.deliver(None,str(error))
        finally:
            self.stop.set()
            if rpc: rpc.close()
def grid_text(frame):
    return '\n'.join(''.join(cell[0] for cell in row if not cell[1]).rstrip() for row in frame['cells'])
