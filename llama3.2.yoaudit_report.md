# YoAuditor Report

## Metadata

- **Repository:** local
- **Analysis Date:** 2026-02-14 05:43:15 UTC
- **Model Used:** `llama3.2:latest`
- **Files Analyzed:** 8
- **Total Issues:** 17
- **Analysis Duration:** 1710.6s

## Table of Contents

- [Metadata](#metadata)
- [Project Overview](#project-overview)
- [Summary](#summary)
- [Issues by File](#issues-by-file)
  - [utils.rs](#utils-rs)
  - [clean_example.py](#clean_example-py)
  - [lib/helper.js](#lib-helper-js)
  - [db.js](#db-js)
  - [auth.py](#auth-py)
  - [main.go](#main-go)
  - [api.js](#api-js)
  - [performance.py](#performance-py)
- [Recommendations](#recommendations)

## Project Overview

Analysis performed by AI agent with tool-calling capabilities.

## Summary

### Issue Severity Breakdown

| 🔴 Critical | 🟠 High | 🟡 Medium | 🟢 Low | **Total** |
|:---:|:---:|:---:|:---:|:---:|
| 0 | 17 | 0 | 0 | **17** |

### Issues by Category

| Category | Count |
|:---|:---:|
| security | 14 |
| performance | 3 |

### Files by Language

| Language | Files |
|:---|:---:|
| Unknown | 8 |

### Most Problematic Files

| File | Issues |
|:---|:---:|
| `utils.rs` | 3 |
| `api.js` | 3 |
| `performance.py` | 3 |
| `lib/helper.js` | 2 |
| `auth.py` | 2 |

## Issues by File

### utils.rs {#utils-rs}

*Language: Unknown | Lines: 0 | Issues: 3*

#### 🟠 **HIGH** security - Potential panic: indexing without bounds check in hot path

**Lines:** 3

**Description:** The function get_item does not perform a bounds check on the index parameter, which could lead to an out-of-bounds access and a potential crash.

> 💡 **Suggestion:** Use the optional method to handle the case where the index is out of bounds.

---

#### 🟠 **HIGH** security - unwrap() can panic on invalid input

**Lines:** 5

**Description:** The function parse_port uses unwrap() to handle parsing errors, which could lead to a panic if the input string is not a valid u16.

> 💡 **Suggestion:** Use a more robust error handling mechanism, such as returning an error value or using a match statement.

---

#### 🟠 **HIGH** security - Division by zero: no check for b == 0

**Lines:** 7

**Description:** The function div does not perform a bounds check on the divisor parameter, which could lead to a division by zero error.

> 💡 **Suggestion:** Add a check for b == 0 before performing the division.

---

### clean_example.py {#clean_example-py}

*Language: Unknown | Lines: 0 | Issues: 1*

#### 🟠 **HIGH** security - False positive: safe_parse_int does not handle all error cases

**Lines:** 2

**Description:** The function safe_parse_int only handles some error cases and may return incorrect results.

> 💡 **Suggestion:** Use a more robust parsing mechanism that handles all possible error cases.

---

### lib/helper.js {#lib-helper-js}

*Language: Unknown | Lines: 0 | Issues: 2*

#### 🟠 **HIGH** security - Global mutable state: cache could grow unbounded

**Lines:** 3

**Description:** The variable cache is declared as global and can grow unbounded without any limits.

> 💡 **Suggestion:** Use a more robust caching mechanism, such as a Least Recently Used (LRU) cache or a cache with a fixed size.

---

#### 🟠 **HIGH** security - XSS: user input inserted into HTML without escaping

**Lines:** 5

**Description:** The function renderHtml inserts the user input text directly into the HTML without any escaping or sanitization.

> 💡 **Suggestion:** Use a more robust HTML escaping mechanism, such as using a library like DOMPurify.

---

### db.js {#db-js}

*Language: Unknown | Lines: 0 | Issues: 1*

#### 🟠 **HIGH** security - Stub for api.js - no real DB

**Lines:** 1

**Description:** The module exports a stub function that returns an empty array instead of performing a database query.

> 💡 **Suggestion:** Use a more realistic stub or mock the database behavior.

---

### auth.py {#auth-py}

*Language: Unknown | Lines: 0 | Issues: 2*

#### 🟠 **HIGH** security - Timing attack: == short-circuits, not constant-time

**Lines:** 2

**Description:** The function check_password uses a direct comparison for the password verification, which could lead to timing attacks.

> 💡 **Suggestion:** Use a more robust comparison mechanism that is constant-time.

---

#### 🟠 **HIGH** security - No rate limiting - brute force possible

**Lines:** 4

**Description:** The function login does not implement any rate limiting for the login attempts, which could lead to brute-force attacks.

> 💡 **Suggestion:** Use a more robust rate limiting mechanism, such as using a token bucket or a sliding window.

---

### main.go {#main-go}

*Language: Unknown | Lines: 0 | Issues: 2*

#### 🟠 **HIGH** security - Defer in loop: file not closed until function returns - leak

**Lines:** 4

**Description:** The function processFiles does not close the file descriptor after use, which could lead to a resource leak.

> 💡 **Suggestion:** Use a defer statement or a finally block to ensure that the file is closed.

---

#### 🟠 **HIGH** security - No validation: empty or missing env could cause issues downstream

**Lines:** 6

**Description:** The function getEnv does not validate the environment variable before returning its value, which could lead to security issues.

> 💡 **Suggestion:** Use a more robust validation mechanism, such as checking for the existence and format of the variable.

---

### api.js {#api-js}

*Language: Unknown | Lines: 0 | Issues: 3*

#### 🟠 **HIGH** security - SQL injection: user input concatenated into query

**Lines:** 3

**Description:** The function getUserByEmail concatenates the user input email directly into the SQL query without any sanitization or escaping.

> 💡 **Suggestion:** Use a more robust parameterized query mechanism, such as using prepared statements.

---

#### 🟠 **HIGH** security - Dangerous: eval executes arbitrary code

**Lines:** 5

**Description:** The function runUserScript uses the eval() function to execute arbitrary code, which could lead to security issues.

> 💡 **Suggestion:** Use a more robust evaluation mechanism, such as using a sandboxed environment or a safer evaluation library.

---

#### 🟠 **HIGH** security - No validation: id could be negative or non-numeric

**Lines:** 7

**Description:** The function parseId does not validate the input id before returning its parsed value.

> 💡 **Suggestion:** Use a more robust validation mechanism, such as checking for the existence and format of the variable.

---

### performance.py {#performance-py}

*Language: Unknown | Lines: 0 | Issues: 3*

#### 🟠 **HIGH** performance - N+1 pattern: one query per user instead of batch

**Lines:** 4

**Description:** The function get_all_orders performs a separate database query for each user in the list, which could lead to performance issues.

> 💡 **Suggestion:** Use a batching mechanism or a single query with multiple parameters to reduce the number of queries.

---

#### 🟠 **HIGH** performance - Blocking I/O in loop - should use async or parallel

**Lines:** 8

**Description:** The function fetch_all performs a blocking I/O operation for each URL in the list, which could lead to performance issues.

> 💡 **Suggestion:** Use an asynchronous or parallel approach to perform the I/O operations.

---

#### 🟠 **HIGH** performance - Inefficient: string concat in loop (O(n^2))

**Lines:** 10

**Description:** The function build_string uses a string concatenation approach that has a time complexity of O(n^2), which could lead to performance issues.

> 💡 **Suggestion:** Use a more efficient string concatenation approach, such as using a StringBuilder or a buffer.

---

## Recommendations

Based on the analysis, here are the top recommendations for improving this codebase:

1. Review all reported issues and prioritize by severity.
2. Address critical and high severity issues first.

---

*Report generated by [YoAuditor](https://github.com/sharafdin/yoauditor)*
