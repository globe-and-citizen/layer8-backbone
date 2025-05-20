const puppeteer = require("puppeteer");

(async () => {
  const browser = await puppeteer.launch({
    headless: true,
    args: ["--no-sandbox"],
  });
  const page = await browser.newPage();
  await page.goto("http://localhost:5173/");

  await page.type('input[name="question"]', "hello");
  await page.keyboard.press("Enter");

  // Wait for the answer to appear in the textarea
  await page.waitForFunction(
    () => {
      const textarea = document.querySelector('textarea[name="answer"]');
      return textarea && textarea.value !== "";
    },
    { timeout: 5000 } // wait up to 5 seconds
  );

  const answer = await page.$eval('textarea[name="answer"]', (el) => el.value);
  console.assert(
    answer === "Hello from Node.js backend! - hello",
    `Expected "Hello from Node.js backend! - hello", got "${answer}"`
  );

  console.log("End-to-end test passed with message from BE:", answer);

  await browser.close();
})();
