import json, time, urllib.request

BASE = "http://localhost:11434/api/chat"
USER = "Напиши короткий стих про кота, 4 строки"
SYS = "Ты — локальный ассистент Tool. Отвечай на языке вопроса, кратко и по делу. Язык ответов: ru. /no_think"

def run(name, model, num_gpu, timeout=240):
    body = {
        "messages": [{"role": "system", "content": SYS}, {"role": "user", "content": USER}],
        "model": model,
        "options": {"num_ctx": 8192, "temperature": 0.3, "num_gpu": num_gpu},
        "stream": False,
    }
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
    print(f"=== {name} | {dt:.1f}s | {ntok} tokens | {ntok/dt if dt else 0:.1f} tok/s | think={'<think>' in content}")
    print(f"  {repr(content[:90])}")

run("qwen3:0.6b CPU (num_gpu=0)", "qwen3:0.6b", 0)
