const express = require("express");
const cors = require("cors");
const bcrypt = require("bcrypt");
const { poems, users } = require("./mock-database.js");
const jwt = require("jsonwebtoken");

const app = express();
const port = 6191;
const SECRET_KEY = "my_very_secret_key";

app.use(express.json());
app.use(cors());


app.get("/healthcheck", (req, res) => {
  console.log("Enpoint for testing");
  console.log("req.body: ", req.body);
  res.send("Bro, ur poems coming soon. Relax a little.");
});

app.get("/", (req, res) => {
  res.json({ message: "Hello there!" });
});

app.get("/poem", (req, res) => {
  const poem_id = parseInt(req.query.id, 10);
  if (isNaN(poem_id)) {
    return res.status(400).json({ error: "Invalid or missing poem ID!" });
  }
  const poem = poems.find((p) => p.id === poem_id);
  if (poem) {
    res.status(200).json(poem);
  } else {
    res.status(404).json({ error: "Poem not found!" });
  }
});

app.post("/register", async (req, res) => {
    console.log("Request body: ", req.body);
  const { username, password } = req.body;

  try {
    const hashedPassword = await bcrypt.hash(password, 10);
    users.push({ username, password: hashedPassword });
    res.status(200).send("User registered successfully!");
  } catch (err) {
    console.log("err: ", err);
    res.status(500).send({ error: "Something went wrong!" });
  }
});

app.post("/login", async (req, res) => {
  //console.log("users: ", users);
  const { username, password } = req.body;
  const user = users.find((u) => u.username === username);
  if (user && (await bcrypt.compare(password, user.password))) {
    const token = jwt.sign({ username }, SECRET_KEY);
    res.status(200).json({ user, token });
  } else {
    res.status(401).json({ error: "Invalid credentials!" });
  }
});

app.listen(port, () => {
  console.log(`Node.js server is running on http://localhost:${port}`);
});