// index.js
const express = require("express");
const cors = require("cors");
const bcrypt = require("bcrypt");
const { poems, users, images } = require("./mock-database.js");
const jwt = require("jsonwebtoken");
const fs = require("fs");
const path = require("path");
const multer = require("multer");
const ClientOAuth2 = require("client-oauth2");

const app = express();
const port = process.env.PORT || 3000;
const SECRET_KEY = process.env.JWT_SECRET || "my_very_secret_key";

app.use(express.json());
app.use(cors());

let inMemoryUsers = users[0];

// Hard-coded variables for now
// Please login as client (layer8/12341234) to http://localhost:5001 and
// replace the layer8secret and layer8Uuid with the values you get from the Layer8 client
const layer8Secret = process.env.LAYER8_SECRET;// "4a73983ffd5fecfd8f6f2792121a6658";
const layer8Uuid = process.env.LAYER8_UUID; // "26d6f8b5-9438-4556-872b-d60535d8d3c8";
const LAYER8_URL = process.env.LAYER8_URL; // "http://52.221.209.158:5001";
const LAYER8_CALLBACK_URL = process.env.LAYER8_CALLBACK_URL; // "http://localhost:3030/oauth2/callback";
const LAYER8_RESOURCE_URL = process.env.LAYER8_RESOURCE_URL; // "http://localhost:5001/api/user";

console.log("LAYER8_URL: ", LAYER8_URL);
console.log("LAYER8_CALLBACK_URL: ", LAYER8_CALLBACK_URL);
console.log("LAYER8_RESOURCE_URL: ", LAYER8_RESOURCE_URL);
console.log("LAYER8_UUID: ", layer8Uuid);
console.log("LAYER8_SECRET: ", layer8Secret);

const layer8Auth = new ClientOAuth2({
  clientId: layer8Uuid,
  clientSecret: layer8Secret,
  accessTokenUri: `${LAYER8_URL}/api/oauth`,
  authorizationUri: `${LAYER8_URL}/authorize`,
  redirectUri: LAYER8_CALLBACK_URL,
  scopes: ["read:user"],
});

// console.log("inMemoryUsers: ", inMemoryUsers);

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
  res.setHeader(
    "Content-Disposition",
    `attachment; filename="${username}_profile${path.extname(filePath)}"`
  );
  res.sendFile(filePath);
});

app.post("/update-user-profile-metadata", async (req, res) => {
  const { email_verified, country, city, phone_number, address } = req.body;
  if (email_verified) {
    inMemoryUsers.metadata.email_verified = true;
  }
  if (country) {
    inMemoryUsers.metadata.country = "Canada";
  }
  if (city) {
    inMemoryUsers.metadata.city = "Vancouver";
  }
  if (phone_number) {
    inMemoryUsers.metadata.phone_number = "1234567890";
  }
  if (address) {
    inMemoryUsers.metadata.address = "123 Main St, Test Address";
  }
  res.status(200).json({ message: "Metadata updated successfully" });
});

app.get("/api/login/layer8/auth", async (req, res) => {
  res.status(200).json({ authURL: layer8Auth.code.getUri() });
});

app.post("/authorization-callback", async (req, res) => {

  const myHeaders = new Headers();
  myHeaders.append("Content-Type", "application/json");

  const raw = JSON.stringify({
    authorization_code: req.body.code,
    redirect_uri: LAYER8_CALLBACK_URL,
    client_oauth_uuid: layer8Uuid,
    client_oauth_secret: layer8Secret,
  });

  const requestOptions = {
    method: "POST",
    headers: myHeaders,
    body: raw,
    redirect: "follow",
  };

  // Variable to store the Layer8 token response
  let layer8TokenResponse;

  await fetch(LAYER8_URL + "/api/token", requestOptions)
    .then((response) => response.text())
    .then((result) => {
      layer8TokenResponse = result;
    })
    .catch((error) => console.error(error));


  // layer8TokenResponse 2:  {"is_success":true,"message":"access token generated successfully","errors":null,"data":{"access_token":"eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjE3NTI3NTc3MjksImlhdCI6MTc1Mjc1NzEyOSwiaXNzIjoiR2xvYmUgYW5kIENpdGl6ZW4iLCJzdWIiOiIyNmRmZDc4ZC1iZTdhLTQzMjEtYmNmYi01OTI3ZGEyMWM3ZmIiLCJVc2VySUQiOjEsIlNjb3BlcyI6ImNvdW50cnksZW1haWxfdmVyaWZpZWQsZGlzcGxheV9uYW1lLGNvbG9yIn0.0Umong9zxiW_wmBVmbtQ2xJyGavOQSDau6Uq22zo6TU","token_type":"bearer","expires_in_minutes":10}}

  const accessToken = JSON.parse(layer8TokenResponse).data.access_token;

  let metadataResponse;

  //  Body :
  //   {
  //     "client_oauth_uuid": "26dfd78d-be7a-4321-bcfb-5927da21c7fb",
  //     "client_oauth_secret": "be3caef54fc0ec0dcd87b0a65cf24f81598243b5f01b4cce6a344718db854fe6"
  //    }

  await fetch(LAYER8_URL + "/api/zk-metadata", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${accessToken}`,
    },
    body: JSON.stringify({
      client_oauth_uuid: layer8Uuid,
      client_oauth_secret: layer8Secret,
    }),
  })
    .then((response) => response.json())
    .then((data) => {
      metadataResponse = data;
    })
    .catch((err) => console.error(err));

  if (metadataResponse.is_success) {
    inMemoryUsers.metadata.email_verified =
      metadataResponse.data.is_email_verified;
    inMemoryUsers.metadata.country = metadataResponse.data.country;
    inMemoryUsers.metadata.display_name = metadataResponse.data.display_name;
    inMemoryUsers.metadata.color = metadataResponse.data.color;
  }

  res.status(200).json({ message: "Layer8 auth successful" });
});

const host = process.env.HOST || "localhost";
app.listen(port, () => {
  console.log(`Node.js server is running on http://${host}:${port}`);
});
