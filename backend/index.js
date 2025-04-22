const express = require('express');
const bodyParser = require('body-parser');
const app = express();
const port = 3000;

app.use(bodyParser.json());

app.post('/path', (req, res) => {
    const requestData = req.body.data;
    console.log(`Received data: ${requestData}`);

    // Process the request and send response back
    const responseData = `Hello from Node.js backend! - ${requestData}`;
    res.status(200).json({ data: responseData });
});

app.listen(port, () => {
    console.log(`Node.js server is running on http://localhost:${port}`);
});
