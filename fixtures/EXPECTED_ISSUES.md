# Expected issues (for manual check)

Use this as a checklist when running YoAuditor on this fixture. The auditor should report issues similar to these.

| File | Category | Issue |
|------|----------|--------|
| `auth.py` | Security | Hardcoded API key / secret |
| `auth.py` | Security | Timing attack in password comparison (non constant-time) |
| `auth.py` | Security | No rate limiting on login (brute force) |
| `api.js` | Security | SQL injection in `getUserByEmail` |
| `api.js` | Security | Dangerous `eval()` on user input |
| `api.js` | Bug / Validation | `parseId` – no validation of query param |
| `utils.rs` | Bug | Possible panic: unchecked index in `get_item` |
| `utils.rs` | Bug | `parse_port`: `unwrap()` can panic |
| `utils.rs` | Bug | `div`: division by zero |
| `main.go` | Bug / Perf | `defer` in loop – resource leak |
| `main.go` | Bug | Error ignored in `processFiles` |
| `main.go` | Code quality | `getEnv` – no validation of empty/missing env |
| `performance.py` | Performance | N+1 query in loop |
| `performance.py` | Performance | Blocking I/O in loop (should be async/parallel) |
| `performance.py` | Performance | Inefficient string concat in loop |
| `lib/helper.js` | Bug / Security | Global mutable cache, unbounded growth |
| `lib/helper.js` | Security | XSS: HTML built from user input without escaping |
| `clean_example.py` | — | (Clean; use to check false positives) |

Run from repo root:

```bash
yoauditor --repo local --local ./fixtures --output yoaudit_report.md
```

Then open `yoaudit_report.md` and compare with the table above.
