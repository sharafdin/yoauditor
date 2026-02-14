# YoAuditor Fixture Audit Results

Results of running YoAuditor on the fixtures with two setups: **local** (small model on your machine) and **cloud** (large model). Both ran on the same code; this report compares them.

## Fixture and Expected Issues

- **Fixture:** `fixtures/` — intentionally flawed code in Python, JavaScript, Go, and Rust.
- **Expected issues:** See [EXPECTED_ISSUES.md](./EXPECTED_ISSUES.md) (17 issues; `clean_example.py` is intentionally clean; `db.js` is an intentional stub).

---

## Local vs Cloud

| | **Local** | **Cloud** |
|--|-----------|-----------|
| **Model** | llama3.2:latest | qwen3-coder:480b-cloud |
| **Where it runs** | On your machine (Ollama) | Cloud API |
| **Size** | Small (~3B) | Large (480B) |
| **Report** | `llama3.2.yoaudit_report.md` | `qwen3-coder.yoaudit_report.md` |
| **Files analyzed** | 8 | 6 |
| **Issues reported** | 17 | 17 |
| **Duration** | ~28.5 min | **~21 s** |
| **Expected issues found** | 15/17 | **17/17** |
| **False positives** | 2 | **0** |
| **Severity spread** | All High | Critical / High / Medium |

---

## Local (Llama 3.2) — small and yet performed well

The **local** run uses a **small, lightweight model** (Llama 3.2) via Ollama. No cloud, no API keys, everything stays on your machine.

- **Found 15 of 17** expected issues — solid for a tiny model.
- Caught SQL injection, eval(), timing attacks, rate limiting, N+1, blocking I/O, string concat, XSS, global cache, defer-in-loop, getEnv validation, and all three utils.rs bugs.
- **Missed:** hardcoded API key in auth.py; ignored error in main.go.
- **False positives:** 2 (clean_example.py, db.js stub). Severity was flat (all High).
- **Runtime:** ~28 min (single-call with full context).

**Verdict:** For a small local model, it performed well. Good choice for offline or privacy-sensitive audits when you can tolerate a couple of misses and some false positives.

---

## Cloud (Qwen3 Coder 480B) — best accuracy and speed

The **cloud** run uses a **large model** (Qwen3 Coder 480B) via a cloud endpoint.

- **Found all 17/17** expected issues.
- **0 false positives.** Severity was well calibrated (2 Critical for SQL injection and eval, then High/Medium).
- **Runtime:** ~21 seconds — much faster than the local run.

**Verdict:** Best accuracy and speed. Use when you want full coverage, no FPs, and fast results and are fine using the cloud.

---

## Coverage vs Expected Issues

| Expected issue | Local (llama3.2) | Cloud (qwen3 480B) |
|----------------|------------------|---------------------|
| auth.py – Hardcoded API key/secret | ❌ Missed | ✅ |
| auth.py – Timing attack | ✅ | ✅ |
| auth.py – No rate limiting | ✅ | ✅ |
| api.js – SQL injection | ✅ | ✅ (Critical) |
| api.js – Dangerous eval() | ✅ | ✅ (Critical) |
| api.js – parseId no validation | ✅ | ✅ |
| utils.rs – Unchecked index, unwrap, div by zero | ✅ All 3 | ✅ All 3 |
| main.go – Defer in loop | ✅ | ✅ |
| main.go – Error ignored | ❌ Missed | ✅ |
| main.go – getEnv no validation | ✅ | ✅ |
| performance.py – N+1, blocking I/O, string concat | ✅ All 3 | ✅ All 3 |
| lib/helper.js – Global cache, XSS | ✅ Both | ✅ Both |

---

## Summary

| Criterion | Local (llama3.2) | Cloud (qwen3 480B) |
|-----------|------------------|---------------------|
| Expected issues found | 15/17 | **17/17** ✅ |
| False positives | 2 | **0** ✅ |
| Severity spread | All High | Critical / High / Medium ✅ |
| Runtime | ~28 min | **~21 s** ✅ |
| Runs offline | ✅ | No |

- **Local:** Small, runs on your machine, performed well for its size (15/17). Use for offline or privacy-first audits.
- **Cloud:** Large, fast, full coverage and no FPs. Use when you want the best accuracy and speed.

---

*Comparison of `llama3.2.yoaudit_report.md` (local) and `qwen3-coder.yoaudit_report.md` (cloud) vs [EXPECTED_ISSUES.md](./EXPECTED_ISSUES.md).*
