import json,urllib.request,re
API="http://127.0.0.1:8787/api/v1"
def http(m,p,b=None,t=None):
    h={"content-type":"application/json"}
    if t:h["authorization"]="Bearer "+t
    r=urllib.request.Request(API+p,data=json.dumps(b).encode() if b is not None else None,method=m,headers=h)
    try:
        x=urllib.request.urlopen(r,timeout=120);d=x.read();return json.loads(d) if d else None
    except urllib.error.HTTPError as e: raise SystemExit(f"{m} {p} {e.code} {e.read()[:300]}")
PW="demo-password-123"
names=["Ada Okafor","Bola Adeyemi","Chidi Nwosu","Dayo Balogun","Efe Eze","Funke Ojo","Gbenga Lawal","Halima Musa","Ife Bello","Jide Umar","Kemi Ayoola","Lara Obi"]
lt=http("POST","/auth/lecturers/register",{"name":"Dr. Amara Eze","email":"amara.eze@university.edu","password":PW})["access_token"]
c=http("POST","/courses",{"code":"BIO201","title":"Plant Biology","academic_session":"2026/2027","semester":"First Semester"},lt)
roster=[{"matric_number":f"BIO/26/{i+1:03d}","full_name":n,"email":f"{n.split()[0].lower()}@university.edu"} for i,n in enumerate(names)]
http("POST",f"/courses/{c['id']}/roster",{"students":roster},lt)
# Past lectures with varied attendance so analytics has history
import random
random.seed(7)
def past(title, n, text):
    s=http("POST","/sessions",{"course_id":c["id"],"title":title},lt)["session"]
    code=s["short_code"]
    for r in roster[:n]:
        j=http("POST",f"/sessions/code/{code}/join",{"matric_number":r["matric_number"],"display_name":r["full_name"]})
        for _ in range(random.choice([1,2,2,2])): http("POST",f"/participants/{j['id']}/heartbeat")
    for sent in re.split(r"(?<=[.])\s+",text): http("POST",f"/sessions/code/{code}/captions",{"text":sent},lt)
    http("POST",f"/sessions/code/{code}/end",None,lt)
    http("POST",f"/sessions/code/{code}/attendance/reconcile",None,lt)
    return code
past("Cells and Organelles",11,"The cell is the basic unit of life. Plant cells have a cell wall, a large vacuole and chloroplasts. The nucleus stores DNA and the mitochondria release energy.")
past("Plant Transport",10,"Water moves up through the xylem by transpiration pull. Sugars move through the phloem by translocation from sources to sinks.")
code=past("Cell Respiration",9,"Cellular respiration releases energy from glucose. Glycolysis happens in the cytoplasm and produces pyruvate. The Krebs cycle occurs in the mitochondrial matrix and the electron transport chain makes most of the ATP.")
# Student account for the phone
http("POST","/auth/students/register",{"matric_number":roster[0]["matric_number"],"display_name":"Ada Okafor","email":"ada@university.edu","password":PW})
print("seeded",c["id"],code)
