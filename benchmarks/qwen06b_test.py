import json, time, urllib.request

BASE = "http://localhost:11434/api/chat"
USER = "Скажи: привет"
SYS_PLAIN = "Ты — локальный ассистент Tool. Отвечай на языке вопроса, кратко и по делу. Язык ответов: ru."

def run(name, model, sysprompt, user, timeout=90):
    body = {
        "messages": [{"role": "system", "content": sysprompt}, {"role": "user", "content": user}],
        "model": model,
        "options": {"num_ctx": 8192, "temperature": 0.3},
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
    print(f"=== {name} | {dt:.1f}s | tokens={resp.get('eval_count')} | open={'<think>' in content} close={'</think>' in content} len={len(content)}")
    print(f"  answer: {repr(content[:120])}")

run("qwen3:0.6b plain (no think)", "qwen3:0.6b", SYS_PLAIN, USER)
run("qwen3:0.6b marker", "qwen3:0.6b", SYS_PLAIN, "/no_think " + USER)
