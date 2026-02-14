// Nested file: global variable, no input sanitization

var cache = {}; // Global mutable state

function getCached(key) {
  return cache[key];
}

function setCached(key, value) {
  cache[key] = value; // No size limit - could grow unbounded
}

function renderHtml(text) {
  // XSS: user input inserted into HTML without escaping
  return "<div>" + text + "</div>";
}
