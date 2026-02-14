# Fixtures

Sample code with **intentional issues** for testing YoAuditor (Python, JavaScript, Go, Rust). From repo root:

```bash
yoauditor --repo local --local ./fixtures --output yoaudit_report.md
```

To compare models, use named outputs:

```bash
yoauditor --repo local --local ./fixtures --output qwen3-coder.yoaudit_report.md
yoauditor --repo local --local ./fixtures --model llama3.2:latest --output llama3.2.yoaudit_report.md
```

| File | Purpose |
|------|--------|
| **[EXPECTED_ISSUES.md](./EXPECTED_ISSUES.md)** | Checklist of issues the auditor should find. |
| **[AUDIT_RESULTS.md](./AUDIT_RESULTS.md)** | Comparison of model runs vs expected issues. |
| **clean_example.py** | Intentionally clean; use to check for false positives. |
| **lib/helper.js** | Nested file; tests scanner recursion and more issues. |

Expected: the auditor should report security issues, bugs, and code quality problems in these files.
