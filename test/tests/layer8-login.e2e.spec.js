const { test, expect } = require("@playwright/test");

test("logs in with Layer8 and stores username", async ({ page }) => {
  const frontendOrigin = "http://localhost:5173";
  let layer8Authenticated = false;

  page.on("dialog", async (dialog) => {
    await dialog.accept();
  });

  await page.addInitScript(() => {
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

      return { close() {} };
    };
  });

  await page.route("**/*", async (route) => {
    const request = route.request();
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
      route.fulfill({
        status,
        headers: corsHeaders,
        body: JSON.stringify(body),
      });

    if (
      ["/me", "/api/l8-login", "/l8-login-callback"].includes(normalizedPath) &&
      method === "OPTIONS"
    ) {
      return route.fulfill({
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

    return route.continue();
  });

  await page.goto("/");
  await page.getByRole("button", { name: "Login With Layer8" }).click();

  await expect(page).toHaveURL(/\/profile$/);
  await expect
    .poll(() => page.evaluate(() => localStorage.getItem("username")))
    .toBe("layer8-user");
});
