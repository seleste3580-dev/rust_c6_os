import sys
MAP={'1':".,?!1@-_/:;()&$%+=#'\"",'2':"abc2",'3':"def3",'4':"ghi4",'5':"jkl5",'6':"mno6",'7':"pqrs7",'8':"tuv8",'9':"wxyz9",'0':" 0"}
class S:
    def __init__(s): s.t=0; s.lines=[]
    def wait(s,ms): s.t+=ms
    def key(s,k,hold=60,long=False):
        s.lines.append(f"{s.t} {k} p")
        if long:
            s.t+=800; s.lines.append(f"{s.t} {k} l"); s.t+=100
        else: s.t+=hold
        s.lines.append(f"{s.t} {k} r"); s.t+=120
    def snap(s,name=None): s.lines.append(f"{s.t} snap"); s.t+=50
    def typ(s,text):
        last=None
        for ch in text:
            for k,v in MAP.items():
                if ch in v:
                    n=v.index(ch)+1
                    if last==k: s.wait(1100)
                    for _ in range(n): s.key(k,40); 
                    last=k;break
def run(build,out):
    s=S(); build(s); open(out,'w').write("\n".join(s.lines)+"\n")
def digits(s,text):
    for ch in text: s.key(ch,40)
S.digits=digits
