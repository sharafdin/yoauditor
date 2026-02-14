// Intentional issues: SQL injection, eval, no validation

const db = require("./db");

function getUserByEmail(email) {
  // SQL injection: user input concatenated into query
  const query = `SELECT * FROM users WHERE email = '${email}'`;
  return db.query(query);
}

function runUserScript(code) {
  // Dangerous: eval executes arbitrary code
  return eval(code);
}

function parseId(req) {
  // No validation: id could be negative or non-numeric
  const id = req.query.id;
  return parseInt(id, 10);
}

module.exports = { getUserByEmail, runUserScript, parseId };
