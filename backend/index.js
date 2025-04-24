const express = require("express");
const cors = require("cors");
const app = express();
const port = 3000;

app.use(express.json());
app.use(
  cors({
    origin: "http://localhost:6191",
    methods: "POST",
    allowedHeaders: ["Content-Type"],
  })
);

app.post("/test-endpoint", (req, res) => {
  console.log("Received request:", req.body); // Log the entire request body
  if (!req.body) {
    return res.status(400).json({ error: "Invalid request payload" });
  }

  const requestData = req.body.data;
  console.log(`Received data: ${requestData}`);

  const responseData = `Hello from Node.js backend! - ${requestData}`;
  console.log(`Sending response: ${responseData}`);
  res.status(200).json({ data: responseData });
});

app.listen(port, () => {
  console.log(`Node.js server is running on http://localhost:${port}`);
});
