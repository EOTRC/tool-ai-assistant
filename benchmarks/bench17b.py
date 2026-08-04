import urllib.request, json, time

def bench(model, num_gpu):
    body = json.dumps({
        "model": model,
        "messages": [{"role": "user", "content": "Назови 3 столицы Европы одним списком через запятую."}],
        "stream": False,
        "options": {"num_gpu": num_gpu, "num_ctx": 8192}
    }).encode()
    t0 = time.time()
    req = urllib.request.Request("http://localhost:11434/api/chat", data=body, headers={"Content-Type": "application/json"})
    r = json.load(urllib.request.urlopen(req, timeout=600))
    dt = (r.get("eval_count") or 0) / (r.get("eval_duration") or 1) * 1e9
    print(f"{model} num_gpu={num_gpu}: {r.get('eval_count')} ток, {dt:.1f} ток/с, всего {time.time()-t0:.1f}s")
    print("   ответ:", r["message"]["content"][:200].replace("\n", " "))

bench("qwen3:1.7b", 0)
bench("qwen3:1.7b", 99)
