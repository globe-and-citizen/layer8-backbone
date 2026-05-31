const puppeteer = require("puppeteer");

(async () => {
  const frontendOrigin = "http://localhost:5173";

  const browser = await puppeteer.launch({
    headless: true,
    args: ["--no-sandbox"],
  });

  const page = await browser.newPage();

  page.on("dialog", async (dialog) => {
    await dialog.accept();
  });

  await page.evaluateOnNewDocument(() => {
    window.open = () => {
      setTimeout(() => {
        window.dispatchEvent(
          new MessageEvent("message", {
            data: {
              redirect_uri: "http://localhost:5173/oauth2/callback",
              code: "mock-layer8-code",
            },
          })
        );
      }, 10);

      return {
        close() {},
      };
    };
  });

  let layer8Authenticated = false;
  await page.setRequestInterception(true);
  page.on("request", (request) => {
    const targetPath = new URL(request.url()).pathname;
    const normalizedPath = targetPath.replace(/^\/undefined/, "");
    const method = request.method();
    const corsHeaders = {
      "access-control-allow-origin": frontendOrigin,
      "access-control-allow-credentials": "true",
      "access-control-allow-headers": "content-type",
      "access-control-allow-methods": "GET,POST,OPTIONS",
      "content-type": "application/json",
    };

    const respondJson = (status, body) =>
      request.respond({
        status,
        headers: corsHeaders,
        body: JSON.stringify(body),
      });

    if (
      ["/me", "/api/l8-login", "/l8-login-callback"].includes(normalizedPath) &&
      method === "OPTIONS"
    ) {
      return request.respond({
        status: 204,
        headers: corsHeaders,
      });
    }

    if (normalizedPath === "/me") {
      if (!layer8Authenticated) {
        return respondJson(401, { error: "unauthorized" });
      }

      return respondJson(200, { user: { username: "layer8-user" } });
    }

    if (normalizedPath === "/api/l8-login") {
      return respondJson(200, {
        authURL: "https://layer8.local/authorize?state=e2e",
      });
    }

    if (normalizedPath === "/l8-login-callback" && method === "POST") {
      const requestBody = request.postData();
      const body = requestBody ? JSON.parse(requestBody) : {};
      if (!body.code) {
        return respondJson(400, { error: "missing code" });
      }

      layer8Authenticated = true;
      return respondJson(200, { profile: { username: "layer8-user" } });
    }

    return request.continue();
  });

  await page.goto(`${frontendOrigin}/`, { waitUntil: "networkidle0" });

  await page.evaluate(() => {
    const button = Array.from(document.querySelectorAll("button")).find((el) =>
      el.textContent?.includes("Login With Layer8")
    );
    if (!button) {
      throw new Error("Layer8 login button not found");
    }

    button.click();
  });

  await page.waitForFunction(() => location.pathname === "/profile", {
    timeout: 10000,
  });

  await page.waitForFunction(
    () => localStorage.getItem("username") === "layer8-user",
    { timeout: 10000 }
  );

  const username = await page.evaluate(() => localStorage.getItem("username"));
  console.assert(
    username === "layer8-user",
    `Expected localStorage username to be "layer8-user", got "${username}"`
  );

  console.log("Layer8 login browser test passed for user:", username);
  await browser.close();
})();
