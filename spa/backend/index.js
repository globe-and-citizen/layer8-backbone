// index.js
const express = require("express");
const cors = require("cors");
const bcrypt = require("bcrypt");
const {poems, users, images} = require("./mock-database.js");
const jwt = require("jsonwebtoken");
const fs = require("fs");
const path = require("path");
const multer = require("multer");
const ClientOAuth2 = require("client-oauth2");
const session = require("express-session");

const app = express();
const port = process.env.PORT || 3000;
const SECRET_KEY = process.env.JWT_SECRET || "my_very_secret_key";

app.use(express.json());
app.use(cors({
    origin: "http://localhost:5173", // EXACT frontend origin
    credentials: true,               // allow cookies
}));
// Session middleware
app.use(
    session({
        name: "demo.spa", // cookie name
        secret: "super-secret-key", // change in production
        resave: false,
        saveUninitialized: false,
        cookie: {
            // httpOnly: true,
            // secure: false, // true if HTTPS
            // sameSite: "lax",
            maxAge: 1000 * 60 * 60, // 1 hour
        },
    })
);

/* =========================
   Logger Middleware
========================= */
const loggerMiddleware = (req, res, next) => {
    const start = Date.now();
    const originalSend = res.send;

    res.send = function (data) {
        console.log("Outgoing Response:", {
            statusCode: res.statusCode,
            duration: `${Date.now() - start}ms`,
            path: req.path,
        });
        return originalSend.apply(res, arguments);
    };

    next();
};

app.use(loggerMiddleware);

/* =========================
   Layer8 Config (UNCHANGED)
========================= */
const layer8Secret = process.env.LAYER8_SECRET;
const layer8Uuid = process.env.LAYER8_UUID;
const LAYER8_URL = process.env.LAYER8_URL;
const LAYER8_CALLBACK_URL = process.env.LAYER8_CALLBACK_URL;

const layer8Client = new ClientOAuth2({
    clientId: layer8Uuid,
    clientSecret: layer8Secret,
    accessTokenUri: `${LAYER8_URL}/api/v1/oauth`,
    authorizationUri: `${LAYER8_URL}/oauth/authorize`,
    redirectUri: LAYER8_CALLBACK_URL,
    state: generateRandomString()
});

/* =========================
   Helper Functions (NEW)
========================= */

async function exchangeLayer8Code(code) {
    const response = await fetch(`${LAYER8_URL}/api/v1/oauth/token`, {
        method: "POST",
        headers: {"Content-Type": "application/json"},
        body: JSON.stringify({
            code,
            redirect_uri: LAYER8_CALLBACK_URL,
            client_id: layer8Uuid,
            client_secret: layer8Secret,
            grant_type: "authorization_code",
        }),
    });

    const text = await response.text();
    return text ? JSON.parse(text) : {};
}

async function fetchLayer8Metadata(accessToken) {
    const response = await fetch(`${LAYER8_URL}/api/v1/oauth/zk-metadata`, {
        method: "POST",
        headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${accessToken}`,
        },
        body: JSON.stringify({
            client_id: layer8Uuid,
            client_secret: layer8Secret,
        }),
    });

    return response.json();
}

function isLoggedIn(req, res) {
    if (!req.session.username) {
        console.log('user not logged in');
        res.status(401).json({ error: "Unauthorized" });
        return
    }
    console.log('logged in user:', req.session.username);
    return req.session.username
}

/* =========================
   File Upload Setup
========================= */

const storage = multer.diskStorage({
    destination: (req, file, cb) => {
        const uploadDir = path.join(__dirname, "uploads");
        if (!fs.existsSync(uploadDir)) {
            fs.mkdirSync(uploadDir, {recursive: true});
        }
        cb(null, uploadDir);
    },
    filename: (req, file, cb) => {
        const username = req.params.username;
        const ext = path.extname(file.originalname);
        cb(null, `${username}_profile${ext}`);
    },
});

const upload = multer({
    storage,
    limits: {fileSize: 1024 * 1024 * 5},
    fileFilter: (req, file, cb) => {
        if (file.mimetype.startsWith("image/")) cb(null, true);
        else cb(new Error("Only image files are allowed!"), false);
    },
});

app.use("/images", express.static(path.join(__dirname, "images")));
app.use("/uploads", express.static(path.join(__dirname, "uploads")));

/* =========================
   Basic Routes
========================= */

app.get("/healthcheck", (req, res) => {
    res.send("Bro, ur poems coming soon. Relax a little.");
});

app.get("/", (req, res) => {
    res.json({message: "Hello there!"});
});

/* =========================
   Poems & Images
========================= */

app.get("/poems", (req, res) => {
    const poem_id = parseInt(req.query.id, 10);

    if (poem_id) {
        const poem = poems.find((p) => p.id === poem_id);
        return poem
            ? res.status(200).json(poem)
            : res.status(404).json({error: "Poem not found!"});
    }

    res.status(200).json(poems);
});

app.get("/images", (req, res) => {
    const image_name = req.query.name;

    if (image_name) {
        const image = images.find((i) =>
            i.name.toLowerCase().includes(image_name.toLowerCase())
        );

        return image
            ? res.status(200).json(image)
            : res.status(404).json({error: "Image not found!"});
    }

    res.status(200).json(images);
});

/* =========================
   Register & Login
========================= */

app.post("/register", async (req, res) => {
    const {username, password} = req.body;

    if (users.find((u) => u.username === username)) {
        return res.status(400).json({error: "Username already exists"});
    }

    const hashedPassword = await bcrypt.hash(password, 10);

    users.push({
        username,
        password: hashedPassword,
        metadata: {
            email_verified: false,
            country: "",
            display_name: "",
            color: "",
        },
    });

    res.status(200).send("User registered successfully!");
});

app.post("/login", async (req, res) => {
    const {username, password} = req.body;
    const user = users.find((u) => u.username === username);

    if (user && (await bcrypt.compare(password, user.password))) {
        const token = jwt.sign({username}, SECRET_KEY);
        return res.status(200).json({user, token});
    }

    res.status(401).json({error: "Invalid credentials!"});
});

app.get("/me", (req, res) => {
    if (!req.session.username) {
        return res.status(401).json({ authenticated: false });
    }

    const user = users.find((u) => u.username === req.session.username);

    res.status(200).json({
        authenticated: true,
        user: user,
    });
});

app.post("/logout", (req, res) => {
    console.log("logout user:", req.session.user);
    req.session.destroy(() => {
        res.clearCookie("demo.spa");
        // res.json({ message: "Logged out" });
        res.status(200).json({ authenticated: false });
    });
});

/* =========================
   Profile
========================= */

app.get("/profile/:username", (req, res) => {
    let username = isLoggedIn(req, res);
    if (!username) return;

    const user = users.find((u) => u.username === req.params.username);
    // if (!user) return res.status(404).json({error: "User not found!"});

    const response = {
        username: user.username,
        metadata: user.metadata || null,
    };

    if (user.metadata?.profilePicture) {
        response.profilePicture =
            `${req.protocol}://${req.get("host")}` +
            user.metadata.profilePicture;
    }

    res.status(200).json(response);
});

app.post(
    "/profile/:username/upload",
    upload.single("profile_pic"),
    (req, res) => {
        const user = users.find((u) => u.username === req.params.username);

        if (!req.file)
            return res.status(400).json({error: "No file uploaded"});

        if (!user) {
            fs.unlinkSync(req.file.path);
            return res.status(404).json({error: "User not found"});
        }

        user.metadata.profilePicture = `/uploads/${req.file.filename}`;

        res.status(200).json({
            message: "Profile picture uploaded successfully",
            path: user.metadata.profilePicture,
        });
    }
);

/* =========================
   OAuth (read:user)
========================= */

function generateRandomString(size = 32) {
    const {randomBytes} = require("crypto");
    return randomBytes(size).toString("base64url");
}

app.get("/api/login/layer8/auth", (req, res) => {
    res.status(200).json({authURL: layer8Client.code.getUri({scopes: ["read:user"]})});
});

app.post("/authorization-callback", async (req, res) => {
    try {
        // const token = req.headers.authorization;
        // const tokenStr = token.replace("Bearer ", "");
        // const payload = JSON.parse(atob(tokenStr.split('.')[1]));
        // const username = payload.username;
        const username = isLoggedIn(req, res);
        if (!username) return;

        let inMemoryUsers = users.find((u) => u.username === username);
        console.log('memo user', username, inMemoryUsers);

        const tokenResp = await exchangeLayer8Code(req.body.code);
        const accessToken = tokenResp?.data?.access_token;
        console.log("accessToken resp", tokenResp);

        if (!accessToken)
            return res.status(400).json({error: "No access token returned"});

        const metadataResponse = await fetchLayer8Metadata(accessToken);
        console.log("metadata", metadataResponse)

        if (metadataResponse.is_success) {
            inMemoryUsers.metadata.email_verified =
                metadataResponse.data.is_email_verified;
            inMemoryUsers.metadata.bio = metadataResponse.data.bio;
            inMemoryUsers.metadata.display_name = metadataResponse.data.display_name;
            inMemoryUsers.metadata.color = metadataResponse.data.color;
        }

        res.status(200).json({
            message: "Layer8 auth successful",
        });
    } catch (err) {
        console.error(err);
        res.status(500).json({error: "Layer8 auth failed"});
    }
});

/* =========================
   OIDC (openid)
========================= */

app.get("/api/l8-login", (req, res) => {
    res.status(200).json({authURL: `${layer8Client.code.getUri({scopes: ["openid"]})}&nonce=${generateRandomString()}`});
});

app.post("/l8-login-callback", async (req, res) => {
    try {
        const tokenResp = await exchangeLayer8Code(req.body.code);
        const idToken = tokenResp?.data?.id_token;
        console.log("tokenResp", tokenResp);

        if (!idToken) return res.status(400).json({error: "No id_token in response"});

        const decoded = jwt.decode(idToken);

        const username =
            decoded?.username ||
            decoded?.email ||
            decoded?.sub;

        if (!username) return res.status(400).json({error: "Invalid ID token"});

        let user = users.find((u) => u.username === username);

        if (!user) {
            user = {
                username,
                password: null,
                metadata: {
                    email_verified: decoded?.email_verified ?? false,
                    display_name: decoded?.display_name || "",
                    color: decoded?.color || "",
                },
            }
            users.push(user);
        }

        // Save user info in session
        req.session.username = username;

        // const token = jwt.sign({username}, SECRET_KEY);

        res.status(200).json({
            message: "Layer8 auth successful",
            profile: user,
            // token,
        });
    } catch (err) {
        console.error(err);
        res.status(500).json({error: "Layer8 login failed"});
    }
});

/* =========================
   Start Server
========================= */

const host = process.env.HOST || "localhost";

app.listen(port, () => {
    console.log(`Node.js server is running on http://${host}:${port}`);
});
