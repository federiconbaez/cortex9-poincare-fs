import csv,sys,collections,re
def pct(v,p):
    v=sorted(v); return 1e3*v[round(p/100*(len(v)-1))]
def faiss(txt):
    d={}
    for l in open(txt):
        m=re.match(r"\| (\S+) \| faiss-flatip \| (\d+) \| ([\d.]+) % \((\d+)/(\d+)\) \| ([\d.]+) \| ([\d.]+) \|",l)
        if m: d[m.group(1)]=(int(m.group(4)),int(m.group(5)),float(m.group(7)))
    return d
for base in sys.argv[1:]:
    rows=collections.defaultdict(lambda:{'ok':0,'n':0,'t':[]})
    for r in csv.DictReader(open(base+'.tsv'),delimiter='\t'):
        k=(r['conjunto'],r['metodo'])
        if r['rep']=='0': rows[k]['ok']+=int(r['ok1']); rows[k]['n']+=1
        else: rows[k]['t'].append(float(r['segundos']))
    F=faiss(base+'.txt')
    print('##',base)
    for conj,met in [("T1'",'fs-similar'),("T2'",'fs-buscar-hibrido'),("T2'",'fs-buscar-bola'),("T1''",'fs-similar-hibrido'),("T1'",'fs-similar-hibrido')]:
        a=rows[(conj,met)]; g=rows[(conj,'rg')]; f=F[conj]
        p=pct(a['t'],50); pr=pct(g['t'],50)
        ra=a['ok']/a['n']; rr=g['ok']/g['n']; rf=f[0]/f[1]
        ok= p<=0.7*pr and p<=0.7*f[2] and ra>=rr and ra>=rf
        print(f"{conj}\t{met}\tR@1 {100*ra:.1f} rg {100*rr:.1f} faiss {100*rf:.1f}\tp50 {p:.4f} rg {pr:.4f} faiss {f[2]:.4f}\tt/rg {p/pr:.3f} t/faiss {p/f[2]:.3f}\t{'PASA' if ok else 'FALLA'}")
