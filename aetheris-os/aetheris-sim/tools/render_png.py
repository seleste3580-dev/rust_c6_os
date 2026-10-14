import sys
from PIL import Image
data=open(sys.argv[1],'rb').read(); out=sys.argv[2]
n=len(data)//2048
def img(buf,tint):
    im=Image.new('RGB',(128,64),(0,0,0))
    px=im.load()
    for p in range(8):
        for x in range(128):
            v=buf[p*128+x]
            for b in range(8):
                if v>>b&1:
                    y=p*8+b
                    px[x,y]= (255,200,0) if (tint and y<16) else (90,170,255)
    return im.resize((256,128),Image.NEAREST)
cols=2; rows=n
sheet=Image.new('RGB',(cols*262, rows*134),(30,30,30))
for i in range(n):
    f=data[i*2048:(i+1)*2048]
    sheet.paste(img(f[:1024],True),(0,i*134)); sheet.paste(img(f[1024:],False),(262,i*134))
sheet.save(out); print(n,"frames")
