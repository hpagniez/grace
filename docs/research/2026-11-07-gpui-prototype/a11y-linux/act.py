import sys, gi
gi.require_version('Atspi','2.0')
from gi.repository import Atspi
Atspi.init()
def find(a, role, name, out):
    try:
        if a.get_role_name()==role and a.get_name()==name: out.append(a)
        for i in range(a.get_child_count()):
            c=a.get_child_at_index(i)
            if c: find(c, role, name, out)
    except Exception: pass
d=Atspi.get_desktop(0); out=[]
for i in range(d.get_child_count()): find(d.get_child_at_index(i), sys.argv[1], sys.argv[2], out)
print('found', len(out))
idx = int(sys.argv[3]) if len(sys.argv)>3 else 0
a=out[idx]; act=a.get_action_iface()
print('do_action ->', act.do_action(0))
