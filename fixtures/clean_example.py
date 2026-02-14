# Intentionally clean(ish) - use to check for false positives.
# Auditor should report little or nothing here.

def safe_parse_int(s: str) -> int | None:
    try:
        return int(s)
    except ValueError:
        return None


def get_item_safe(items: list, index: int):
    if 0 <= index < len(items):
        return items[index]
    return None
