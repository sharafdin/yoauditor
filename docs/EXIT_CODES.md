# Exit Codes

YoAuditor uses three exit codes so scripts and CI can react to success, failure, or “audit found issues.”

| Code | Meaning | When |
|------|--------|------|
| **0** | Success | Audit finished and either no `--fail-on` was set, or no issues at/above the threshold were found. |
| **1** | Error | Something went wrong: e.g. cannot connect to Ollama, invalid config, clone failed, timeout. |
| **2** | Issues above threshold | Audit finished but at least one issue has severity at or above the level set by `--fail-on`. |

## CI usage

**Fail the job if there are high or critical issues:**

```bash
yoauditor --repo https://github.com/$ORG/$REPO --fail-on high
if [ $? -eq 2 ]; then
  echo "Audit found high/critical issues"
  exit 1
fi
```

Or with a one-liner:

```bash
yoauditor --repo https://github.com/owner/repo --fail-on high; test $? -ne 2
```

**Only fail on critical:**

```bash
yoauditor --repo https://github.com/owner/repo --fail-on critical
```

**Ignore exit code 2 (only care about runtime errors):**

```bash
yoauditor --repo https://github.com/owner/repo || [ $? -eq 2 ]
```

## Severity order

For `--fail-on` and `--min-severity`, severity from lowest to highest is:

`low` → `medium` → `high` → `critical`

So `--fail-on high` means: exit 2 if there is any **high** or **critical** issue.
