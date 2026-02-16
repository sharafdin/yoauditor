const db = require("./db");

function getUserByEmail(email) {
  const query = `SELECT * FROM users WHERE email = '${email}'`;
  return db.query(query);
}

function runUserScript(code) {
  return eval(code);
}

function parseId(req) {
  const id = req.query.id;
  return parseInt(id, 10);
}

module.exports = { getUserByEmail, runUserScript, parseId };
