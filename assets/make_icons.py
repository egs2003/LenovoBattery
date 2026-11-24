import struct, io, sys
from PIL import Image, ImageDraw

BG=(30,35,39,255); FG=(232,234,237,255)
GREEN=(52,195,143,255); AMBER=(245,165,36,255); RED=(239,68,68,255)

def line(d,s,x1,y1,x2,y2,w,col):
    d.line([(x1*s,y1*s),(x2*s,y2*s)],fill=col,width=round(w*s))
    r=w*s/2
    for x,y in((x1,y1),(x2,y2)):
        d.ellipse([x*s-r,y*s-r,x*s+r,y*s+r],fill=col)

def outline(d,s,x,y,w,h,r,sw,col):
    d.rounded_rectangle([(x-sw/2)*s,(y-sw/2)*s,(x+w+sw/2)*s,(y+h+sw/2)*s],radius=(r+sw/2)*s,fill=col)
    d.rounded_rectangle([(x+sw/2)*s,(y+sw/2)*s,(x+w-sw/2)*s,(y+h-sw/2)*s],radius=max(r-sw/2,0)*s,fill=BG)

def draw(kind,size,small):
    ss=size*8 if size<256 else size*4
    s=ss/200
    im=Image.new("RGBA",(ss,ss),(0,0,0,0)); d=ImageDraw.Draw(im)
    d.rounded_rectangle([0,0,ss-1,ss-1],radius=44*s,fill=BG)
    sw=10 if small else 7
    if kind=="battery":
        outline(d,s,30,64,124,72,12,sw,FG)
        d.rounded_rectangle([159*s,84*s,171*s,116*s],radius=5*s,fill=FG)
        d.rounded_rectangle([(30+sw/2+2)*s,(64+sw/2+2)*s,125*s,(136-sw/2-2)*s],radius=6*s,fill=GREEN)
        if small:
            line(d,s,125,46,125,154,6,AMBER)
        else:
            y=46
            while y<=154:
                line(d,s,125,y,125,y+2,5,AMBER); y+=12
    else:
        outline(d,s,28,40,144,94,12,sw,FG)
        line(d,s,100,134,100,156,sw,FG); line(d,s,70,160,130,160,sw,FG)
        d.ellipse([(75-21)*s,(87-21)*s,(75+21)*s,(87+21)*s],fill=RED)
        bw=8 if small else 6
        for x,a,b in((119,79,95),(133,71,103),(147,77,97)):
            line(d,s,x,a,x,b,bw,FG)
    return im.resize((size,size),Image.LANCZOS)

def build(kind,outdir):
    sizes=[16,24,32,48,64,128,256]
    imgs=[]
    for n in sizes:
        im=draw(kind,n,n<=32)
        b=io.BytesIO(); im.save(b,"PNG"); imgs.append((n,b.getvalue()))
    hdr=struct.pack("<HHH",0,1,len(imgs)); off=6+16*len(imgs); ent=b""; data=b""
    for n,p in imgs:
        ent+=struct.pack("<BBBBHHII",n%256,n%256,0,0,1,32,len(p),off+len(data)); data+=p
    open(f"{outdir}/icon.ico","wb").write(hdr+ent+data)
    draw(kind,128,False).save(f"{outdir}/icon.png")
    open(f"{outdir}/icon.rgba","wb").write(draw(kind,128,False).tobytes())

import os
for k,o in(("battery","/home/claude/lb/assets"),("recorder","/home/claude/rec/assets")):
    os.makedirs(o,exist_ok=True); build(k,o)
# Vorschau
sheet=Image.new("RGBA",(560,300),(255,255,255,255))
x=10
for k in("battery","recorder"):
    sheet.paste(draw(k,256,False),(x,10),draw(k,256,False)); 
    xx=x
    for n in(48,32,16):
        im=draw(k,n,n<=32); sheet.paste(im,(xx,276-n if False else 10+256+ (0)),im) if False else None
    x+=275
x=10
for k in("battery","recorder"):
    pass
sheet.save("/home/claude/work/sheet.png")
