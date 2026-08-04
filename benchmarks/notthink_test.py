import json, time, urllib.request

BASE = "http://localhost:11434/api/chat"
USER = "Скажи: привет"

def run(name, model, sysprompt, user, think_field=None, timeout=90):
    body = {
        "messages": [{"role": "system", "content": sysprompt}, {"role": "user", "content": user}],
        "model": model,
        "options": {"num_ctx": 8192, "temperature": 0.3},
        "stream": False,
    }
    if think_field is not None:
        body["think"] = think_field
    req = urllib.request.Request(BASE, data=json.dumps(body).encode("utf-8"), headers={"Content-Type": "application/json"})
    t0 = time.time()
    try:
        resp = json.loads(urllib.request.urlopen(req, timeout=timeout).read())
    except Exception as e:
        print(f"=== {name} | ERROR: {e}")
        return
    dt = time.time() - t0
    content = resp.get("message", {}).get("content", "")
    has_open = "<think>" in content
    has_close = "</think>" in content
    # check if reasoning leaked without opening tag
    print(f"=== {name} | {dt:.1f}s | tokens={resp.get('eval_count')} | open={has_open} close={has_close} len={len(content)}")
    print(f"  answer: {repr(content[:120])}")

SYS_NOTHINK = "Ты — локальный ассистент Tool. Отвечай на языке вопроса, кратко и по делу. Язык ответов: ru. /no_think"
SYS_PLAIN = "Ты — локальный ассистент Tool. Отвечай на языке вопроса, кратко и по делу. Язык ответов: ru."

# qwen3.5:0.8b
run("qwen3.5:0.8b sys+marker", "qwen3.5:0.8b", SYS_NOTHINK, USER)
run("qwen3.5:0.8b marker+user", "qwen3.5:0.8b", SYS_PLAIN, "/no_think " + USER)
# qwen3:4b
run("qwen3:4b sys+marker", "qwen3:4b", SYS_NOTHINK, USER)
run("qwen3:4b marker+user", "qwen3:4b", SYS_PLAIN, "/no_think " + USER)
run("qwen3:4b no think field", "qwen3:4b", SYS_PLAIN, USER)
