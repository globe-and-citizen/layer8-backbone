// index.js
const express = require("express");
const cors = require("cors");
const bcrypt = require("bcrypt");
const { poems, users, images } = require("./mock-database.js");
const jwt = require("jsonwebtoken");
const fs = require('fs');
const path = require('path');

const app = express();
const port = 6191;
const SECRET_KEY = "my_very_secret_key";

app.use(express.json());
app.use(cors());

// Serve static images
app.use('/images', express.static(path.join(__dirname, 'images')));

// Health check endpoint
app.get("/healthcheck", (req, res) => {
  res.send("Bro, ur poems coming soon. Relax a little.");
});

app.get("/", (req, res) => {
  res.json({ message: "Hello there!" });
});

// Updated poem endpoint
app.get("/poems", (req, res) => {
  const poem_id = parseInt(req.query.id, 10);
  
  if (poem_id) {
    // Return single poem if ID is provided
    const poem = poems.find((p) => p.id === poem_id);
    if (poem) {
      res.status(200).json(poem);
    } else {
      res.status(404).json({ error: "Poem not found!" });
    }
  } else {
    // Return all poems if no ID
    res.status(200).json(poems);
  }
});

// New images endpoint
app.get("/images", (req, res) => {
  const image_id = parseInt(req.query.id, 10);
  
  if (image_id) {
    // Return single image if ID is provided
    const image = images.find((img) => img.id === image_id);
    if (image) {
      res.status(200).json(image);
    } else {
      res.status(404).json({ error: "Image not found!" });
    }
  } else {
    // Return all images if no ID
    res.status(200).json(images);
  }
});

// New profile endpoint
app.get("/profile/:username", (req, res) => {
  const { username } = req.params;
  const user = users.find((u) => u.username === username);
  
  if (user) {
    res.status(200).json({
      username: user.username,
      metadata: user.metadata || null
    });
  } else {
    res.status(404).json({ error: "User not found!" });
  }
});

app.post("/register", async (req, res) => {
  const { username, password } = req.body;

  // Check if user already exists
  if (users.find(u => u.username === username)) {
    return res.status(400).json({ error: "Username already exists" });
  }

  try {
    const hashedPassword = await bcrypt.hash(password, 10);
    users.push({ 
      username, 
      password: hashedPassword,
      metadata: null // New users have no metadata
    });
    res.status(200).send("User registered successfully!");
  } catch (err) {
    console.log("err: ", err);
    res.status(500).send({ error: "Something went wrong!" });
  }
});

app.post("/login", async (req, res) => {
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