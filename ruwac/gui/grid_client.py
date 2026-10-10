#!/usr/bin/env python3
"""Native GTK feasibility client: paints canonical cells, never parses VT bytes."""
import argparse, json, os, signal, sys, time
from pathlib import Path
from bridge import Rpc, Worker, grid_text

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--socket',required=True)
    parser.add_argument('--session',action='append',default=[])
    parser.add_argument('--self-test-report',type=Path)
    args=parser.parse_args()
    import gi
    gi.require_version('Gtk','4.0'); gi.require_version('Adw','1'); gi.require_version('GLibUnix','2.0')
    from gi.repository import Gtk, Adw, Gdk, GLib, GLibUnix, Pango, PangoCairo
    gi.require_foreign('cairo')
    if not Gtk.init_check(): raise SystemExit('GUI unavailable: no native display')
    palette=['#000000','#cd0000','#00cd00','#cdcd00','#0000ee','#cd00cd','#00cdcd','#e5e5e5','#7f7f7f','#ff0000','#00ff00','#ffff00','#5c5cff','#ff00ff','#00ffff','#ffffff']
    def rgb(value, foreground):
        if isinstance(value,list): return tuple(x/255 for x in value)
        if value is None or value>=256: return (0.9,0.9,0.9) if foreground else (.06,.06,.07)
        if value<16:
            h=palette[value]; return tuple(int(h[i:i+2],16)/255 for i in (1,3,5))
        if value>=232: return ((8+10*(value-232))/255,)*3
        n=value-16; levels=[0,95,135,175,215,255]
        return (levels[n//36]/255,levels[(n//6)%6]/255,levels[n%6]/255)
    class View(Gtk.Box):
        def __init__(self, session):
            super().__init__(orientation=Gtk.Orientation.VERTICAL,spacing=2)
            self.session=session; self.frame=None; self.target=None; self.drawn=0; self.error=None; self.prefix=False; self.font=Pango.FontDescription.from_string('Monospace 11')
            self.label=Gtk.Label(label='Connecting…',xalign=0); self.append(self.label)
            self.area=Gtk.DrawingArea(hexpand=True,vexpand=True,focusable=True)
            self.area.set_content_width(800); self.area.set_content_height(400)
            self.area.set_draw_func(self.draw); self.append(self.area)
            self.paintable=Gtk.WidgetPaintable.new(self.area)
            self.worker=Worker(args.socket,session,self.deliver)
            key=Gtk.EventControllerKey(); key.connect('key-pressed',self.key); self.area.add_controller(key)
            click=Gtk.GestureClick(); click.set_button(0); click.connect('pressed',self.click); self.area.add_controller(click)
            self.area.connect('resize',self.resize)
        def deliver(self,frame,error):
            GLib.idle_add(self.update,frame,error)
        def update(self,frame,error):
            if error: self.error=error;self.label.set_text('Disconnected: '+error); return False

            if self.target not in [p['id'] for p in frame['panes']]: self.target=frame['active_pane']
            self.frame=frame; self.label.set_text(frame['name']+' — Ctrl+A commands; PTYs remain in daemon'); self.area.queue_draw(); return False
        def metrics(self):
            layout=self.area.create_pango_layout('M'); layout.set_font_description(self.font)
            return layout.get_pixel_size()
        def draw(self,area,cr,width,height):
            self.drawn+=1
            cr.set_source_rgb(.06,.06,.07);cr.paint()
            if not self.frame: return
            cw,ch=self.metrics(); y=0
            layout=area.create_pango_layout(''); layout.set_font_description(self.font)
            for pane in self.frame['panes']:
                grid=pane['grid']
                for r,row in enumerate(grid['cells']):
                    for c,cell in enumerate(row):
                        text,spacer,fg,bg,bold,inverse=cell
                        if inverse: fg,bg=bg,fg
                        cr.set_source_rgb(*rgb(bg,False));cr.rectangle(c*cw,y+r*ch,cw,ch);cr.fill()
                        if not spacer:
                            cr.set_source_rgb(*rgb(fg,True));cr.move_to(c*cw,y+r*ch)
                            layout.set_font_description(Pango.FontDescription.from_string('Monospace Bold 11') if bold else self.font)
                            layout.set_text(text,-1);PangoCairo.show_layout(cr,layout)
                if pane['id']==self.frame['active_pane'] and grid['cursor']['visible']:
                    cursor=grid['cursor'];cr.set_source_rgb(.6,.7,.9);cr.rectangle(cursor['col']*cw,y+cursor['row']*ch,cw,ch);cr.stroke()
                y+=grid['rows']*ch
                cr.set_source_rgb(.3,.3,.4);cr.rectangle(0,y,width,1);cr.fill();y+=ch
        def resize(self,area,width,height):
            cw,ch=self.metrics()
            try:self.worker.send('client.resize',{'rows':max(4,min(100,height//max(1,ch)-2)),'cols':max(10,min(200,width//max(1,cw)))})
            except Exception as error: self.label.set_text(str(error))
        def command(self,method,params):
            try:self.worker.send(method,params)
            except Exception as error:self.label.set_text(str(error))
        def key(self,controller,keyval,keycode,state):
            if not self.frame:return True
            pane=self.target; session=self.session
            control=bool(state&Gdk.ModifierType.CONTROL_MASK); alt=bool(state&Gdk.ModifierType.ALT_MASK)
            char=chr(Gdk.keyval_to_unicode(keyval)) if Gdk.keyval_to_unicode(keyval) else ''
            if control and state&Gdk.ModifierType.SHIFT_MASK and char.lower() in ('c','v'):
                self.label.set_text('Desktop copy/paste is not yet implemented in this native spike');return True
            name=Gdk.keyval_name(keyval)
            if self.prefix:
                self.prefix=False
                if char=='d':self.worker.close();self.label.set_text('Detached; daemon PTYs preserved');return True
                if char=='c':self.command('window.create',{'session':session})
                elif char in ('n','p'):self.command('window.cycle',{'session':session,'previous':char=='p'})
                elif char=='S':self.command('pane.create',{'window':self.frame['window']})
                elif char=='a' or (control and char.lower()=='a'):self.command('client.input',{'pane':pane,'kind':'keyboard','text':'\x01'})
                elif char==']':self.command('buffer.paste',{'pane':pane})
                elif name=='Tab':
                    panes=self.frame['panes'];i=next(i for i,p in enumerate(panes) if p['id']==pane);self.command('pane.select',{'pane':panes[(i+1)%len(panes)]['id']})
                elif char in ('?','"',':','[', '|') or char.isdigit():
                    self.label.set_text('Use the TUI for copy/help/window selection; this is a bounded native paint spike')
                return True
            if control and char.lower()=='a':self.prefix=True;return True
            special={'Return':'\r','KP_Enter':'\r','BackSpace':'\x7f','Tab':'\t','Escape':'\x1b','Up':'\x1b[A','Down':'\x1b[B','Right':'\x1b[C','Left':'\x1b[D','Home':'\x1b[H','End':'\x1b[F','Delete':'\x1b[3~','Page_Up':'\x1b[5~','Page_Down':'\x1b[6~'}
            data=special.get(name,char)
            if control and len(char)==1 and '@'<=char.upper()<='_':data=chr(ord(char.upper())&31)
            if data=='\x00':self.label.set_text('NUL input is unsupported by this text protocol');return True
            if alt:data='\x1b'+data
            if data:self.command('client.input',{'pane':pane,'kind':'keyboard','text':data})
            return True
        def click(self,gesture,n,x,y):
            self.area.grab_focus()
            if not self.frame:return
            _,ch=self.metrics();offset=0
            for p in self.frame['panes']:
                bottom=offset+(p['grid']['rows']+1)*ch
                if offset<=y<bottom:self.target=p['id'];self.command('pane.select',{'pane':p['id']});break
                offset=bottom
    app=Adw.Application(application_id='si.z4.ruwac.spike',flags=0)
    views=[];window=None;stack=None;test_phase=0;started=time.monotonic()
    def close(*_):
        for view in views:view.worker.close()
        return False
    def activate(application):
        nonlocal window,stack
        sessions=list(args.session)
        if not sessions:
            rpc=Rpc(args.socket)
            try:
                for label in ('Terminal 1','Terminal 2'):sessions.append(rpc.call('session.create',{'name':label})['session'])
            finally:rpc.close()
        window=Adw.ApplicationWindow(application=application,title='RUWAC — native terminal spike',default_width=1000,default_height=720)
        box=Gtk.Box(orientation=Gtk.Orientation.VERTICAL); header=Adw.HeaderBar();box.append(header)
        stack=Adw.ViewStack(vexpand=True,hexpand=True,enable_transitions=False);switcher=Adw.ViewSwitcher(stack=stack);header.set_title_widget(switcher)
        for i,session in enumerate(sessions):
            view=View(session);views.append(view);stack.add_titled(view,str(i),'Terminal '+str(i+1))
        box.append(stack);window.set_content(box);window.connect('close-request',close);window.present()
        if args.self_test_report:GLib.timeout_add(100,self_test)
    def self_test():
        nonlocal test_phase
        if time.monotonic()-started>20:
            args.self_test_report.write_text(json.dumps({'status':'FAIL','reason':'Timed out waiting for native canonical frames','errors':[v.error for v in views],'frames':[bool(v.frame) for v in views],'drawn':[v.drawn for v in views],'mapped':window.get_mapped(),'sizes':[(v.area.get_width(),v.area.get_height()) for v in views]}));close();app.quit();return False
        if not views or any(not view.frame for view in views):return True
        if time.monotonic()-started<1:return True
        if test_phase==0:stack.set_visible_child(views[-1]);test_phase=1;return True
        if test_phase==1:
            if views[-1].drawn==0:return True
            stack.set_visible_child(views[0]);test_phase=2;return True
        if views[0].drawn==0:return True
        if test_phase==2:
            view=views[0];view.click(None,1,2,2)
            for char in 'GUI!':view.key(None,Gdk.unicode_to_keyval(ord(char)),0,Gdk.ModifierType(0))
            view.key(None,Gdk.KEY_a,0,Gdk.ModifierType.CONTROL_MASK)
            view.key(None,Gdk.KEY_a,0,Gdk.ModifierType.CONTROL_MASK)
            test_phase=time.monotonic();return True
        if time.monotonic()-test_phase<1:return True
        snapshot=Gtk.Snapshot.new();views[0].paintable.snapshot(snapshot,views[0].area.get_width(),views[0].area.get_height())
        node=snapshot.to_node()
        if node is None:return True
        texture=window.get_renderer().render_texture(node,None)
        texture.save_to_png(str(args.self_test_report.with_suffix('.png')))
        report={'status':'PASS','display':type(Gdk.Display.get_default()).__name__,'native_window':True,'tabs':len(views),'panes':[len(v.frame['panes']) for v in views],'sessions':[v.session for v in views],'rendered_text':[[grid_text(p['grid']) for p in v.frame['panes']] for v in views],'image_width':texture.get_width(),'image_height':texture.get_height(),'synthetic_gtk_keyboard':'GUI! and nested Ctrl+A via GTK handler; receiver verified separately','physical_input':'BLOCKED','rdp':'BLOCKED','parser_owner':'daemon only','native_draw_calls':[v.drawn for v in views],'mapped':window.get_mapped()}
        args.self_test_report.write_text(json.dumps(report,indent=2)+'\n');close();app.quit();return False
    app.connect('activate',activate)
    GLibUnix.signal_add(GLib.PRIORITY_DEFAULT,signal.SIGTERM,lambda:(close(),app.quit(),False)[-1])
    app.run([])
if __name__=='__main__':main()
