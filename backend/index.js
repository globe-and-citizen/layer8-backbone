const express = require("express");
const cors = require("cors");
const morgan = require('morgan'); // Import the morgan middleware
const winston = require('winston'); // Import Winston for flexible logging
const expressWinston = require('express-winston'); // Middleware for integrating with Express
const fs = require('fs'); // Import the file system module
const path = require('path'); // Import the path module for file paths

const app = express();
const port = 3000;

// Create a write stream (stream) to append data into 'log.txt'
const logStream = fs.createWriteStream(path.join(__dirname, 'log.txt'), { flags: 'a' });

app.use(express.json());
app.use(
  cors({
    origin: "*",
    methods: "*",
    allowedHeaders: ["Content-Type"],
  })
);

// Morgan logging middleware for request information
app.use(morgan('combined', { stream: logStream }));

// Winston logging middleware for detailed error and info logs
expressWinston.responseWhitelist.push('body'); // Include the response body in the logs
app.use(expressWinston.logger({
  transports: [
    new winston.transports.Console(),
    new winston.transports.File({ filename: 'log.txt' })
  ],
  format: winston.format.combine(
    winston.format.colorize(),
    winston.format.json()
  ),
  meta: true, // Include metadata in the logs
  msg: "HTTP {{req.method}} {{req.url}} {{res.statusCode}} {{res.responseTime}}ms", // Custom log format
  expressFormat: true, // Use the default Express/morgan request formatting
  colorize: false, // Color the text in the console (optional)
}));

app.get("/health", (req, res) => {
  res.status(200).json({ status: "OK" });
});

app.post("/test-endpoint", (req, res) => {
  console.log("Received request:", req.body); // Log the entire request body to the console as well
  if (!req.body) {
    return res.status(400).json({ error: "Invalid request payload" });
  }

  const requestData = req.body.data;
  console.log(`Received data: ${requestData}`);

  const responseData = `Hello from Node.js backend! - ${requestData}`;
  console.log(`Sending response: ${responseData}`);
  res.status(200).json({ data: responseData });
});

app.post("/init-tunnel", (req, res) => {
  console.log("Received request:", req.body); // Log the entire request body to the console as well
  if (!req.body) {
    return res.status(400).json({ error: "Invalid request payload" });
  }
  res.status(200).json({ data: "OK" });
});

app.use(expressWinston.errorLogger({
  transports: [
    new winston.transports.Console(),
    new winston.transports.File({ filename: 'log.txt' })
  ],
  format: winston.format.combine(
    winston.format.colorize(),
    winston.format.json()
  ),
}));

app.listen(port, () => {
  console.log(`Node.js server is running on http://localhost:${port}`);
});
