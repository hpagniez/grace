import gi, sys, time
gi.require_version('Atspi','2.0')
from gi.repository import Atspi, GLib
Atspi.init()
t0=time.time()
def cb(e):
    try: n=e.source.get_name(); r=e.source.get_role_name()
    except Exception: n=r='?'
    print('%.2f %s [%s] %r %s %s' % (time.time()-t0, e.type, r, n, e.detail1, e.any_data if not hasattr(e.any_data,'get_name') else e.any_data.get_name()), flush=True)
l=Atspi.EventListener.new(cb)
for t in ['object:','window:','focus:']: l.register(t)
GLib.timeout_add_seconds(int(sys.argv[1]), lambda: Atspi.event_quit())
Atspi.event_main()
