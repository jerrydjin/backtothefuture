#!/usr/bin/env python3
"""Complete station-POI scan of committed OSM; no simulated outcomes."""
import json, xml.etree.ElementTree as ET
m=json.load(open('maps/sf-zsfg.json'))
root=ET.parse('maps/sf-zsfg.osm').getroot()
nodes={n.attrib['id']:(float(n.attrib['lat']),float(n.attrib['lon'])) for n in root.findall('node')}
found=[]
for e in list(root.findall('node'))+list(root.findall('way')):
    tags={t.attrib['k']:t.attrib['v'] for t in e.findall('tag')}
    if tags.get('amenity')!='fire_station':continue
    pts=[nodes[n.attrib['ref']] for n in e.findall('nd')] if e.tag=='way' else [nodes[e.attrib['id']]]
    if len(pts)>2:
        # Planar polygon centroid; sufficient at this footprint's metre scale.
        origin=pts[0]
        local=[(p[0]-origin[0],p[1]-origin[1]) for p in pts]
        cross=[a[1]*b[0]-b[1]*a[0] for a,b in zip(local,local[1:])]
        area=sum(cross)
        lat=origin[0]+sum((a[0]+b[0])*c for a,b,c in zip(local,local[1:],cross))/(3*area)
        lon=origin[1]+sum((a[1]+b[1])*c for a,b,c in zip(local,local[1:],cross))/(3*area)
    else:lat,lon=pts[0]
    qualifies=(max(p[0] for p in pts)>=37.750 and min(p[0] for p in pts)<=37.764 and max(p[1] for p in pts)>=-122.416 and min(p[1] for p in pts)<=-122.400)
    import math
    nearest=min(m['nodes'],key=lambda n:((n['lat']-lat)**2+((n['lon']-lon)*math.cos(math.radians(lat)))**2,n['id']))
    found.append(dict(osm_type=e.tag,osm_id=int(e.attrib['id']),name=tags.get('name'),address=tags.get('addr:housenumber','')+' '+tags.get('addr:street',''),lat=lat,lon=lon,qualifies=qualifies,access_node=nearest['id'],source='https://sf-fire.org/fire-station/san-francisco-fire-station-7'))
print(json.dumps(dict(osm='maps/sf-zsfg.osm',source_extent=[-122.416,37.750,-122.400,37.764],all_fire_station_pois=found),indent=2))
