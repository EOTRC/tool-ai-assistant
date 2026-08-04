import json, time, urllib.request

BASE = "http://localhost:11434/api/chat"
body = {
    "messages": [
        {"role": "system", "content": "Ты ассистент. Отвечай кратко."},
        {"role": "user", "content": "Напиши длинный текст на тему: наука и технологии, минимум 300 слов"},
    ],
    "model": "qwen3.5:0.8b",
    "options": {"num_ctx": 8192, "temperature": 0.3},
    "stream": False,
    "think": False,
}
req = urllib.request.Request(BASE, data=json.dumps(body).encode("utf-8"), headers={"Content-Type": "application/json"})
t0 = time.time()
r = json.loads(urllib.request.urlopen(req, timeout=240).read())
dt = time.time() - t0
content = r.get("message", {}).get("content", "")
ntok = r.get("eval_count", 0)
print(f"wall={dt:.1f}s tokens={ntok} rate={ntok/dt if dt else 0:.1f} tok/s think={'<think>' in content or '</think>' in content}")
print("start:", repr(content[:100]))
