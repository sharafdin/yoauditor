# Intentional issues: N+1 pattern, blocking in loop, inefficient concat

def get_all_orders(user_ids):
    orders = []
    for uid in user_ids:
        # N+1: one query per user instead of batch
        orders.extend(db.query("SELECT * FROM orders WHERE user_id = ?", uid))
    return orders

def fetch_all(urls):
    results = []
    for url in urls:
        # Blocking I/O in loop - should use async or parallel
        resp = requests.get(url)
        results.append(resp.json())
    return results

def build_string(parts):
    # Inefficient: string concat in loop (O(n^2))
    out = ""
    for p in parts:
        out += p
    return out
