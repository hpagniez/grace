import sys, gi
gi.require_version('Atspi','2.0')
from gi.repository import Atspi
Atspi.init()
maxn = int(sys.argv[1]) if len(sys.argv) > 1 else 400
count = [0]
def states(a):
    ss = a.get_state_set()
    names = []
    for s in ['FOCUSABLE','FOCUSED','SELECTABLE','SELECTED','EXPANDABLE','EXPANDED','ENABLED','SENSITIVE','SHOWING','VISIBLE','ACTIVE','CHECKED']:
        if ss.contains(getattr(Atspi.StateType, s)): names.append(s.lower())
    return names
def walk(a, depth):
    if count[0] >= maxn: return
    count[0] += 1
    try:
        role = a.get_role_name(); name = a.get_name(); desc = a.get_description()
    except Exception as e:
        print('  '*depth + 'ERR', e); return
    extra = []
    try:
        act = a.get_action_iface()
        if act: extra.append('actions=' + ','.join(act.get_action_name(i) for i in range(act.get_n_actions())))
    except Exception: pass
    try:
        t = a.get_text_iface()
        if t: extra.append('text=%r' % t.get_text(0, min(t.get_character_count(), 60)))
    except Exception: pass
    try:
        attrs = a.get_attributes()
        if attrs: extra.append('attrs=%s' % dict(attrs))
    except Exception: pass
    try:
        c = a.get_component_iface()
        if c:
            e = c.get_extents(Atspi.CoordType.WINDOW); extra.append('box=%d,%d %dx%d' % (e.x, e.y, e.width, e.height))
    except Exception: pass
    print('  '*depth + '[%s] %r %s %s %s' % (role, name, ('desc=%r' % desc) if desc else '', states(a), ' '.join(extra)))
    n = a.get_child_count()
    for i in range(n):
        ch = a.get_child_at_index(i)
        if ch is not None: walk(ch, depth+1)
d = Atspi.get_desktop(0)
for i in range(d.get_child_count()):
    app = d.get_child_at_index(i)
    if app is None: continue
    if len(sys.argv) > 2 and sys.argv[2] not in (app.get_name() or ''): 
        print('app', repr(app.get_name())); continue
    walk(app, 0)
print('nodes printed', count[0])
