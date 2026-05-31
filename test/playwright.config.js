const { defineConfig } = require("@playwright/test");

module.exports = defineConfig({
  testDir: "./tests",
  fullyParallel: false,
  timeout: 30_000,
  use: {
    baseURL: "http://localhost:5173",
    headless: true,
    launchOptions: {
      args: ["--no-sandbox"],
    },
  },
});
