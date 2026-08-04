import json, time, urllib.request

BASE = "http://localhost:11434/api/chat"
body = {"messages": [{"role": "system", "content": "Ты — локальный ассистент Tool. Отвечай на языке вопроса, кратко и по делу. Язык ответов: ru."}, {"role": "user", "content": "Расскажи кратко, что такое Термодинамика"}], "model": "llama3.2:latest", "options": {"num_ctx": 8192, "temperature": 0.3}, "stream": True, "think": False}
req = urllib.request.Request(BASE, data=json.dumps(body).encode("utf-8"), headers={"Content-Type": "application/json"})
t0 = time.time()
r = urllib.request.urlopen(req, timeout=120)
last = None
for line in r:
    if not line.strip():
        continue
    last = json.loads(line)
dt = time.time() - t0
print(f"wall={dt:.1f}s tokens={last.get('eval_count')} rate={last.get('eval_count',0)*1e9/max(last.get('eval_duration',1),1):.1f} tok/s")
