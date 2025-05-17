const puppeteer = require('puppeteer');

(async () => {
  const browser = await puppeteer.launch({headless: true});
  const page = await browser.newPage();
  await page.goto('http://localhost:5173/');

  await page.type('input[name="question"]', 'hello');
  await page.keyboard.press('Enter');

  const answer = await page.$eval('textarea[name="answer"]', el => el.value);
  console.assert(answer === 'Hello from Node.js backend! - hello', `Expected "Hello from Node.js backend! - hello", got "${answer}"`);

  await browser.close();
})();
