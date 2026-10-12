import json, sys, math, glob, os
geo_dir, communes_path, out_dir = sys.argv[1], sys.argv[2], sys.argv[3]
def dp(pts, tol):
    # iterative Douglas-Peucker
    keep = [False]*len(pts); keep[0] = keep[-1] = True
    stack = [(0, len(pts)-1)]
    while stack:
        i0, i1 = stack.pop()
        a, b = pts[i0], pts[i1]
        dmax, idx = 0, None
        dx, dy = b[0]-a[0], b[1]-a[1]
        L = math.hypot(dx, dy)
        for i in range(i0+1, i1):
            p = pts[i]
            d = math.hypot(p[0]-a[0], p[1]-a[1]) if L == 0 else abs(dy*p[0]-dx*p[1]+b[0]*a[1]-b[1]*a[0])/L
            if d > dmax: dmax, idx = d, i
        if idx is not None and dmax > tol:
            keep[idx] = True; stack += [(i0, idx), (idx, i1)]
    return [p for p, k in zip(pts, keep) if k]
def rings(geom):
    polys = geom['coordinates'] if geom['type'] == 'MultiPolygon' else [geom['coordinates']]
    return [poly[0] for poly in polys]
rows = []
deps = json.load(open(os.path.join(geo_dir, 'deps.geojson')))
feats = [(f, 0.01) for f in deps['features']]
for p in sorted(glob.glob(os.path.join(geo_dir, 'dom-*.geojson'))):
    feats.append((json.load(open(p)), 0.004))
for f, tol in feats:
    code, nom = f['properties']['code'], f['properties']['nom']
    out = []
    for r in rings(f['geometry']):
        s = dp(r, tol)
        # drop slivers: fewer than 4 points or a tiny islet
        if len(s) < 4: continue
        xs = [p[0] for p in s]; ys = [p[1] for p in s]
        if max(xs)-min(xs) < tol*2 and max(ys)-min(ys) < tol*2: continue
        out.append(';'.join(f'{p[0]:.3f},{p[1]:.3f}' for p in s))
    assert out, code
    rows.append((code, nom, '|'.join(out)))
rows.sort()
with open(os.path.join(out_dir, 'departements.tsv'), 'w') as fh:
    for code, nom, rr in rows:
        fh.write(f'{code}\t{nom}\t{rr}\n')
communes = json.load(open(communes_path))
seen = set(); lines = []
valid = {c for c, _, _ in rows}
for c in communes:
    dep = c['codeDepartement']
    if dep not in valid: continue
    lon, lat = c['centre']['coordinates']
    pop = c.get('population') or 0
    for cp in c.get('codesPostaux', []):
        k = (cp, c['nom'])
        if k in seen: continue
        seen.add(k)
        nom = c['nom'].replace('\t', ' ')
        lines.append(f'{cp}\t{nom}\t{lon:.3f}\t{lat:.3f}\t{dep}\t{pop}')
lines.sort()
with open(os.path.join(out_dir, 'communes.tsv'), 'w') as fh:
    fh.write('\n'.join(lines) + '\n')
print(len(rows), 'departements', sum(r[2].count(';')+r[2].count('|')+1 for r in rows), 'points;', len(lines), 'communes')
