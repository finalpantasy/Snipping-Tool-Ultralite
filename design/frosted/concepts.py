"""Headless visual studies. No app windows, screenshots or product behavior changes."""
from pathlib import Path
from html import escape
OUT=Path(__file__).resolve().parent

def text(x,y,s,size=13,color='#f3f3f5',weight=400,anchor='start',opacity=1):
    return f'<text x="{x}" y="{y}" font-family="Segoe UI" font-size="{size}" font-weight="{weight}" fill="{color}" text-anchor="{anchor}" opacity="{opacity}">{escape(s)}</text>'
def rect(x,y,w,h,r=0,fill='#ffffff',opacity=1,stroke='none',sw=1):
    return f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{r}" fill="{fill}" fill-opacity="{opacity}" stroke="{stroke}" stroke-width="{sw}"/>'
ICONS={
 'crop':'<path d="M3 9V3h6M15 3h6v6M21 15v6h-6M9 21H3v-6"/>',
 'plus':'<path d="M12 5v14M5 12h14"/>',
 'down':'<path d="m8 10 4 4 4-4"/>',
 'clock':'<circle cx="12" cy="12" r="8.5"/><path d="M12 7v5l3 2"/>',
 'window':'<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M3 9h18"/>',
 'copy':'<rect x="8" y="8" width="12" height="13" rx="2"/><path d="M15 8V3H4v13h4"/>',
 'save':'<path d="M5 3h12l4 4v14H3V3h2Z"/><path d="M7 3v6h10V3M7 21v-7h10v7"/>',
 'upload':'<path d="M12 16V3m-5 5 5-5 5 5M4 15v6h16v-6"/>',
 'settings':'<circle cx="12" cy="12" r="3"/><path d="m9 3-.6 2.5-2.2 1.3L3.7 6l-1.5 2.6L4 10.5v3L2.2 15.4l1.5 2.6 2.5-.8 2.2 1.3L9 21h6l.6-2.5 2.2-1.3 2.5.8 1.5-2.6L20 13.5v-3l1.8-1.9L20.3 6l-2.5.8-2.2-1.3L15 3Z"/>',
 'dots':'<circle cx="5" cy="12" r=".6"/><circle cx="12" cy="12" r=".6"/><circle cx="19" cy="12" r=".6"/>',
 'check':'<path d="m5 12 4 4L19 6"/>',
 'pen':'<path d="m5 16-1 5 5-1L21 8l-5-5L5 16Z M13 6l5 5"/>',
 'marker':'<path d="m4 17 4 4 5-5-4-4-5 5Z M9 12l8-9 5 5-9 8M3 22h8"/>',
 'text':'<path d="M4 4h16M12 4v17M8 21h8M4 4v3M20 4v3"/>',
 'shape':'<rect x="3" y="3" width="12" height="12" rx="2"/><circle cx="16" cy="16" r="5"/>',
 'blur':'<rect x="3" y="3" width="18" height="18" rx="3"/><path d="M9 3v18M15 3v18M3 9h18M3 15h18"/>',
 'arrow':'<path d="M4 20 20 4M10 4h10v10"/>',
 'undo':'<path d="M8 4 3 9l5 5M3 9h10a7 7 0 1 1 0 14"/>',
 'redo':'<path d="m16 4 5 5-5 5M21 9H11a7 7 0 1 0 0 14"/>',
 'min':'<path d="M5 12h14"/>',
 'max':'<rect x="6" y="6" width="12" height="12"/>',
 'close':'<path d="m6 6 12 12M18 6 6 18"/>',
}
def icon(name,x,y,size=18,color='#dedee4',sw=1.65,opacity=1):
    return f'<g transform="translate({x},{y}) scale({size/24})" fill="none" stroke="{color}" stroke-width="{sw}" stroke-linecap="round" stroke-linejoin="round" opacity="{opacity}">{ICONS[name]}</g>'
def rule(x,y,w,opacity=.08):
    return f'<path d="M{x} {y}h{w}" stroke="white" stroke-opacity="{opacity}"/>'
def button(x,y,w,label,name=None,primary=False,disabled=False):
    fill='#0b82f4' if primary else '#ffffff'
    bg=1 if primary else .065
    color='#ffffff' if not disabled else '#96939f'
    s=rect(x,y,w,32,7,fill,bg)
    if name:s+=icon(name,x+11,y+8,16,color)+text(x+34,y+21,label,13,color,500)
    else:s+=text(x+w/2,y+21,label,13,color,500,'middle')
    return s

WALL='''<g id="wallpaper">
 <rect width="1200" height="850" fill="#111218"/>
 <path d="M-100 580C90 730 265 654 433 530S699 409 954 590L1320 890V1080H-100Z" fill="#22283a"/>
 <path d="M-180 430C91 648 157 611 347 455S659 271 859 413L1350 770V840L887 514C679 367 580 384 404 532S99 755-180 570Z" fill="url(#ribbon)"/>
 <path d="M-130 394C100 589 148 541 322 392S604 223 850 380L1320 690" fill="none" stroke="#565571" stroke-opacity=".2" stroke-width="8"/>
 <path d="M1200-180C945 65 1049 209 794 263S408 141 192-50" fill="none" stroke="#353544" stroke-opacity=".18" stroke-width="180"/>
 </g>'''
def begin(w,h,x,y,pw,ph,tint=.76):
    return f'''<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">
    <defs>
    <linearGradient id="ribbon" x1="0" y1="1" x2="1" y2="0"><stop stop-color="#252d48"/><stop offset=".46" stop-color="#645347"/><stop offset=".8" stop-color="#36313e"/></linearGradient>
    <linearGradient id="edge" x1="0" y1="0" x2="0" y2="1"><stop stop-color="white" stop-opacity=".16"/><stop offset=".3" stop-color="white" stop-opacity=".03"/><stop offset="1" stop-color="white" stop-opacity=".06"/></linearGradient>
    <filter id="glass" x="-20%" y="-25%" width="140%" height="150%"><feGaussianBlur stdDeviation="30"/></filter>
    <filter id="shadow" x="-30%" y="-40%" width="160%" height="190%"><feGaussianBlur stdDeviation="24"/></filter>
    <clipPath id="shell"><rect x="{x}" y="{y}" width="{pw}" height="{ph}" rx="14"/></clipPath>
    </defs>{WALL}<use href="#wallpaper"/>
    <rect x="{x}" y="{y+20}" width="{pw}" height="{ph}" rx="14" fill="black" fill-opacity=".55" filter="url(#shadow)"/>
    <g clip-path="url(#shell)"><use href="#wallpaper" filter="url(#glass)"/>{rect(x,y,pw,ph,14,'#242329',tint)}{rect(x,y,pw,42,0,'#ffffff',.018)}</g>
    <rect x="{x+.5}" y="{y+.5}" width="{pw-1}" height="{ph-1}" rx="13.5" fill="none" stroke="url(#edge)"/>
    '''
def chrome(x,y,w):
    return icon('crop',x+18,y+14,16,'#f0f0f2',1.7)+text(x+44,y+26,'Snipping Tool UL',13,'#ecebf0',500)+icon('min',x+w-115,y+13,16,'#c7c4ce',1.3)+icon('max',x+w-78,y+13,16,'#c7c4ce',1.3)+icon('close',x+w-41,y+13,16,'#c7c4ce',1.4)
def cap(key,x,y,w):
    return rect(x,y,w,24,5,'white',.04, '#ffffff12')+text(x+w/2,y+16,key,11,'#c4c0ce',400,'middle')

def capture():
    x,y,w,h=200,116,660,414
    s=begin(1060,680,x,y,w,h,.73)+chrome(x,y,w)+rule(x,y+42,w,.055)
    s+=button(x+18,y+57,84,'New','plus',True)
    s+=button(x+114,y+57,128,'Rectangle','crop')+icon('down',x+224,y+66,14,'#b6b3bf')
    s+=button(x+250,y+57,95,'None','clock')+icon('down',x+328,y+66,14,'#b6b3bf')
    s+=button(x+w-153,y+57,96,'Upload','upload',disabled=True)+icon('settings',x+w-39,y+64,18,'#c3c0ca')
    s+=icon('crop',x+w/2-16,y+174,32,'#a7a1b1',1.25)
    s+=text(x+w/2,y+237,'Select an area to capture',15,'#e2dfe7',400,'middle')
    s+=cap('Ctrl',x+w/2-65,y+255,39)+cap('Shift',x+w/2-21,y+255,45)+cap('S',x+w/2+29,y+255,25)
    s+=rule(x+18,y+h-43,w-36,.06)
    s+=icon('copy',x+21,y+h-28,14,'#a8a3b3',1.5)+text(x+44,y+h-16,'Copies immediately',11,'#bcb7c7')
    s+=text(x+w-22,y+h-16,'Preview opens automatically',11,'#bcb7c7',400,'end')
    s+=text(200,576,'Capture window',13,'#dedbe4',500)+text(200,599,'Graphite glass · quiet controls · familiar Windows behavior',12,'#9d97aa')
    s+=text(200,645,'DESIGN STUDY  /  glass simulated for review',10,'#868093')
    return s+'</svg>'

def document(x,y):
    s=rect(x,y,644,354,0,'#eeeae3',1)
    s+=text(x+38,y+42,'Weekend route',25,'#25262a',600)
    s+=text(x+38,y+69,'A few things to check before heading out.',14,'#73716f')
    s+=f'<path d="M{x+38} {y+92}h568" stroke="#d7d2cb"/>'
    rows=[('09:00','Coffee & a slow start'),('10:30','Take the riverside trail'),('12:00','Lunch near the old bridge'),('14:00','Leave time for the lookout')]
    for i,(time,label) in enumerate(rows):
        cy=y+127+i*43
        s+=text(x+40,cy,time,14,'#8a857d',500)+text(x+120,cy,label,17,'#37383b')
    s+=rect(x+119,y+151,233,23,2,'#efd778',.46)
    s+=text(x+120,y+170,'Take the riverside trail',17,'#37383b')
    s+=f'<path d="M{x+452} {y+178}C{x+420} {y+145} {x+392} {y+141} {x+369} {y+157}m12-20-12 20 23 2" fill="none" stroke="#df6258" stroke-width="3.2" stroke-linecap="round" stroke-linejoin="round"/>'
    s+=text(x+38,y+323,'Trail map + water bottle',13,'#85807a')
    return s

def editor():
    x,y,w,h=120,62,940,654
    s=begin(1180,808,x,y,w,h,.63)+chrome(x,y,w)+rule(x,y+42,w,.055)
    s+=button(x+18,y+57,82,'New','plus',True)
    s+=button(x+112,y+57,128,'Rectangle','crop')+icon('down',x+222,y+66,14,'#b6b3bf')
    s+=button(x+248,y+57,95,'None','clock')+icon('down',x+326,y+66,14,'#b6b3bf')
    s+=button(x+w-309,y+57,82,'Copy','copy')+button(x+w-219,y+57,80,'Save','save')+button(x+w-131,y+57,96,'Upload','upload')
    s+=icon('dots',x+w-26,y+64,18,'#d7d3dd')
    s+=rect(x+18,y+108,w-36,444,8,'#0c0d10',.37)
    s+=document(x+(w-644)/2,y+152)
    # Annotation strip uses the same glass grammar, with functions grouped by task.
    tx,ty,tw=x+(w-612)/2,y+568,612
    s+=rect(tx,ty,tw,46,10,'#e6dfef',.085)
    s+=icon('undo',tx+15,ty+14,17,'#c9c4d1')+icon('redo',tx+52,ty+14,17,'#8f8999')
    s+=f'<path d="M{tx+89} {ty+12}v22" stroke="white" stroke-opacity=".09"/>'
    s+=rect(tx+103,ty+7,34,32,7,'#3b87ec',.26)
    for i,name in enumerate(['pen','marker','text','arrow','shape','blur','crop']):
        s+=icon(name,tx+111+i*45,ty+14,18,'#d6e7ff' if i==0 else '#d5cfde',1.65)
    s+=f'<path d="M{tx+430} {ty+12}v22" stroke="white" stroke-opacity=".09"/>'
    s+=rect(tx+450,ty+15,16,16,8,'#ee776b')+icon('down',tx+472,ty+15,15,'#bdb6c9')+icon('dots',tx+514,ty+14,18,'#d5cfde')
    s+=text(tx+tw-15,ty+29,'100%',11,'#c9c3d2',400,'end')
    s+=icon('check',x+22,y+h-23,12,'#9ac8ac',2)+text(x+42,y+h-12,'Copied to clipboard',11,'#bdb6ca')
    s+=text(x+w-22,y+h-12,'644 × 354',11,'#aaa3b7',400,'end')
    s+=text(x,753,'Annotation preview',13,'#dedbe4',500)+text(x,776,'The capture is the focus. Tools stay compact; Upload stays one click away.',12,'#9d97aa')
    s+=text(x+w,776,'Illustrative capture · simulated glass',10,'#868093',400,'end')
    return s+'</svg>'

if __name__=='__main__':
    for name,svg in [('capture-frosted',capture()),('editor-frosted',editor())]:
        (OUT/(name+'.svg')).write_text(svg,encoding='utf-8')
    print('Two concept SVGs generated. Production code unchanged.')
