// index.js
const express = require("express");
const cors = require("cors");
const bcrypt = require("bcrypt");
const { poems, users, images } = require("./mock-database.js");
const jwt = require("jsonwebtoken");
const fs = require("fs");
const path = require("path");
const multer = require("multer");

const app = express();
const port = 3000;
const SECRET_KEY = "my_very_secret_key";

app.use(express.json());
app.use(cors());

// Configure storage for uploaded files
const storage = multer.diskStorage({
  destination: (req, file, cb) => {
    const uploadDir = path.join(__dirname, "uploads");
    // Create uploads directory if it doesn't exist
    if (!fs.existsSync(uploadDir)) {
      fs.mkdirSync(uploadDir, { recursive: true });
    }
    cb(null, uploadDir);
  },
  filename: (req, file, cb) => {
    // Use username as the filename
    const username = req.params.username;
    const ext = path.extname(file.originalname);
    cb(null, `${username}_profile${ext}`);
  },
});

const upload = multer({
  storage: storage,
  limits: { fileSize: 1024 * 1024 * 5 }, // 5MB limit
  fileFilter: (req, file, cb) => {
    if (file.mimetype.startsWith("image/")) {
      cb(null, true);
    } else {
      cb(new Error("Only image files are allowed!"), false);
    }
  },
});

// Serve static images
app.use("/images", express.static(path.join(__dirname, "images")));

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
  const image_name = req.query.name;

  if (image_name) {
    // Get all images and search using if contains name
    const image = images.find((i) =>
      i.name.toLowerCase().includes(image_name.toLowerCase())
    );
    if (image) {
      res.status(200).json(image);
    } else {
      res.status(404).json({ error: "Image not found!" });
    }
  } else {
    // Return all images if no name
    res.status(200).json(images);
  }
});

app.post("/register", async (req, res) => {
  const { username, password } = req.body;

  // Check if user already exists
  if (users.find((u) => u.username === username)) {
    return res.status(400).json({ error: "Username already exists" });
  }

  try {
    const hashedPassword = await bcrypt.hash(password, 10);
    users.push({
      username,
      password: hashedPassword,
      metadata: null, // New users have no metadata
    });
    res.status(200).send("User registered successfully!");
  } catch (err) {
    console.log("err: ", err);
    res.status(500).send({ error: "Something went wrong!" });
  }
});

app.post("/login", async (req, res) => {
  console.log("reached login endpoint");
  const { username, password } = req.body;
  const user = users.find((u) => u.username === username);

  if (user && (await bcrypt.compare(password, user.password))) {
    const token = jwt.sign({ username }, SECRET_KEY);
    res.status(200).json({ user, token });
  } else {
    res.status(401).json({ error: "Invalid credentials!" });
  }
});

app.post(
  "/profile/:username/upload",
  upload.single("profile_pic"),
  (req, res) => {
    const { username } = req.params;

    if (!req.file) {
      return res
        .status(400)
        .json({ error: "No file uploaded or invalid file type" });
    }

    // Find the user
    const user = users.find((u) => u.username === username);
    if (!user) {
      // Clean up the uploaded file if user doesn't exist
      fs.unlinkSync(req.file.path);
      return res.status(404).json({ error: "User not found" });
    }

    // Update user metadata with profile picture path
    if (!user.metadata) {
      user.metadata = {};
    }

    // Store relative path to the image
    user.metadata.profilePicture = `/uploads/${req.file.filename}`;

    res.status(200).json({
      message: "Profile picture uploaded successfully",
      path: user.metadata.profilePicture,
    });
  }
);

// Serve uploaded files statically
app.use("/uploads", express.static(path.join(__dirname, "uploads")));

// Update the existing profile endpoint to include profile picture
app.get("/profile/:username", (req, res) => {
  const { username } = req.params;
  const user = users.find((u) => u.username === username);

  if (user) {
    const response = {
      username: user.username,
      metadata: user.metadata || null,
    };

    // If profile picture exists, include full URL
    if (user.metadata?.profilePicture) {
      response.profilePicture = `${req.protocol}://${req.get("host")}${
        user.metadata.profilePicture
      }`;
    }

    res.status(200).json(response);
  } else {
    res.status(404).json({ error: "User not found!" });
  }
});

// Add this new endpoint for downloading profile pictures
app.get("/download-profile/:username", (req, res) => {
  const { username } = req.params;
  const user = users.find((u) => u.username === username);

  if (!user || !user.metadata?.profilePicture) {
    return res.status(404).json({ error: "Profile picture not found!" });
  }

  const filePath = path.join(__dirname, user.metadata.profilePicture);
  
  // Set headers to force download
  res.setHeader('Content-Disposition', `attachment; filename="${username}_profile${path.extname(filePath)}"`);
  res.sendFile(filePath);
});

app.listen(port, () => {
  console.log(`Node.js server is running on http://localhost:${port}`);
});
