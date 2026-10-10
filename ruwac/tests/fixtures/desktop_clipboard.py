#!/usr/bin/env python3
"""Independent GDK clipboard peer. Run only in the owned test compositor."""
import gi
for name,version in [('Gtk','4.0'),('Gdk','4.0')]:gi.require_version(name,version)
from gi.repository import Gtk,Gdk,GLib
from pathlib import Path
import hashlib,json,sys
root=Path(sys.argv[1]);Gtk.init();display=Gdk.Display.get_default();window=Gtk.Window(title='RUWAC clipboard fixture');window.set_default_size(300,100);window.set_child(Gtk.Label(label='Generated clipboard fixtures only'));window.present();loop=GLib.MainLoop();sequence=0
(root/'peer-ready.json').write_text(json.dumps({'display':type(display).__name__}))
def tick():
 global sequence
 try:q=json.loads((root/'peer-request.json').read_text())
 except (FileNotFoundError,ValueError):return True
 if q['seq']<=sequence:return True
 sequence=q['seq'];clip=display.get_primary_clipboard() if q.get('primary') else display.get_clipboard()
 def reply(data):
  temp=root/'peer-response.tmp';temp.write_text(json.dumps({'seq':q['seq'],'result':data}));temp.replace(root/'peer-response.json')
 if q['action'] in ('set-text','set-png'):
  payload=(root/q['file']).read_bytes();mime='image/png' if q['action']=='set-png' else 'text/plain;charset=utf-8'
  provider=Gdk.ContentProvider.new_for_bytes(mime,GLib.Bytes.new(payload));clip.set_content(provider);reply({'offered_bytes':len(payload),'sha256':hashlib.sha256(payload).hexdigest(),'mime':mime})
 elif q['action']=='read-text':
  def read(c,result):
   try:
    text=c.read_text_finish(result);data=text.encode();reply({'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest()})
   except Exception as e:reply({'error':str(e)})
  clip.read_text_async(None,read)
 elif q['action']=='quit':reply({'quit':True});loop.quit();return False
 return True
GLib.timeout_add(20,tick);loop.run()
