def get_all_orders(user_ids):
    orders = []
    for uid in user_ids:
        orders.extend(db.query("SELECT * FROM orders WHERE user_id = ?", uid))
    return orders

def fetch_all(urls):
    results = []
    for url in urls:
        resp = requests.get(url)
        results.append(resp.json())
    return results

def build_string(parts):
    out = ""
    for p in parts:
        out += p
    return out
