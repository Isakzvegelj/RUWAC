#!/usr/bin/env python3
"""Private-bus notification receiver; never use on an owner's session bus."""
import json, sys
from pathlib import Path
from gi.repository import Gio, GLib
root=Path(sys.argv[1]);messages=[]
xml='''<node><interface name="org.freedesktop.Notifications">
<method name="Notify"><arg type="s" direction="in"/><arg type="u" direction="in"/><arg type="s" direction="in"/><arg type="s" direction="in"/><arg type="s" direction="in"/><arg type="as" direction="in"/><arg type="a{sv}" direction="in"/><arg type="i" direction="in"/><arg type="u" direction="out"/></method>
<method name="GetCapabilities"><arg type="as" direction="out"/></method>
<method name="GetServerInformation"><arg type="s" direction="out"/><arg type="s" direction="out"/><arg type="s" direction="out"/><arg type="s" direction="out"/></method>
<method name="CloseNotification"><arg type="u" direction="in"/></method>
</interface></node>'''
def call(connection,sender,path,interface,method,params,invocation):
 if method=='Notify':
  data=params.unpack();messages.append({'summary':data[3],'body':data[4]});(root/'notifications.json').write_text(json.dumps(messages));invocation.return_value(GLib.Variant('(u)',(len(messages),)))
 elif method=='GetCapabilities':invocation.return_value(GLib.Variant('(as)',(['body'],)))
 elif method=='GetServerInformation':invocation.return_value(GLib.Variant('(ssss)',('RUWAC fixture','RUWAC','1','1.2')))
 else:invocation.return_value(None)
def acquired(connection,name):
 connection.register_object('/org/freedesktop/Notifications',Gio.DBusNodeInfo.new_for_xml(xml).interfaces[0],call,None,None)
def ready(connection,name): (root/'notification-ready').write_text('ready')
owner=Gio.bus_own_name(Gio.BusType.SESSION,'org.freedesktop.Notifications',Gio.BusNameOwnerFlags.NONE,acquired,ready,None)
GLib.MainLoop().run()
