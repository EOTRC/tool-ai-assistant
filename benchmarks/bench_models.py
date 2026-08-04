import json, time, urllib.request

BASE = "http://localhost:11434/api/chat"
SYS = "Ты — локальный ассистент Tool. Отвечай на языке вопроса, кратко и по делу. Язык ответов: ru."
USER = "Скажи: привет"

def run(name, model, think=None, timeout=180):
    body = {"messages": [{"role": "system", "content": SYS}, {"role": "user", "content": USER}], "model": model, "options": {"num_ctx": 8192, "temperature": 0.3}, "stream": False}
    if think is not None:
        body["think"] = think
    req = urllib.request.Request(BASE, data=json.dumps(body).encode("utf-8"), headers={"Content-Type": "application/json"})
    t0 = time.time()
    try:
        resp = json.loads(urllib.request.urlopen(req, timeout=timeout).read())
    except Exception as e:
        print(f"=== {name} | ERROR: {e}")
        return
    dt = time.time() - t0
    content = resp.get("message", {}).get("content", "")
    ntok = resp.get("eval_count", 0)
    rate = ntok / dt if dt else 0
    print(f"=== {name} | {dt:.1f}s | {ntok} tokens | {rate:.1f} tok/s | think={'<think>' in content or '</think>' in content}")
    print("  answer:", repr(content[:150]))

run("llama3.2:3b", "llama3.2:latest")
run("qwen3.5:0.8b think:false", "qwen3.5:0.8b", think=False)
run("qwen3.5:0.8b no think", "qwen3.5:0.8b")
run("qwen3:4b think:false", "qwen3:4b", think=False)
