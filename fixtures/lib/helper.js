var cache = {};

function getCached(key) {
  return cache[key];
}

function setCached(key, value) {
  cache[key] = value;
}

function renderHtml(text) {
  return "<div>" + text + "</div>";
}
