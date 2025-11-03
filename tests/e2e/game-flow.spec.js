// @ts-check
const { test, expect } = require('@playwright/test');

/**
 * Test: Complete game flow from lobby to first action
 *
 * This test validates:
 * 1. Server is running and accessible
 * 2. Lobby page loads correctly
 * 3. Game can be started with proper API format
 * 4. Content-Type headers are correct
 * 5. JavaScript executes without errors
 * 6. UI updates after actions
 */
test('complete game flow', async ({ page }) => {
  // Track console errors
  const consoleErrors = [];
  page.on('console', msg => {
    if (msg.type() === 'error') {
      consoleErrors.push(msg.text());
    }
  });

  // Navigate to homepage
  await page.goto('/');

  // Verify lobby loads
  await expect(page.locator('h1')).toContainText('Axiomind Poker');
  await expect(page.locator('.tagline')).toContainText('Heads-up no-limit hold\'em');

  // Wait for htmx to load lobby
  await page.waitForSelector('h2:has-text("Game Lobby")', { timeout: 5000 });

  // Verify lobby content
  await expect(page.locator('h2')).toContainText('Game Lobby');
  await expect(page.locator('button:has-text("Start Game")')).toBeVisible();

  // Intercept API request to verify format
  const requestPromise = page.waitForRequest(request =>
    request.url().includes('/api/sessions') &&
    request.method() === 'POST'
  );

  // Start game
  await page.click('button:has-text("START GAME")');

  // Verify API request
  const request = await requestPromise;

  // Critical: Verify Content-Type (this was the bug)
  const contentType = request.headers()['content-type'];
  expect(contentType).toContain('application/json');

  // Verify payload structure
  const body = JSON.parse(request.postData());
  expect(body).toHaveProperty('level');
  expect(body).toHaveProperty('opponent_type');

  // Verify opponent_type format (ai:baseline, NOT "AI")
  expect(body.opponent_type).toMatch(/^(ai:[\w]+|human)$/);

  // Wait for game table to appear
  await page.waitForSelector('.poker-table', { timeout: 10000 });

  // Verify game state is displayed
  await expect(page.locator('.poker-table')).toBeVisible();

  // Check no JavaScript errors occurred
  expect(consoleErrors).toHaveLength(0);
});

/**
 * Test: Verify static files load correctly
 */
test('static assets load', async ({ page }) => {
  // Set up response listeners BEFORE navigation
  const cssPromise = page.waitForResponse(
    response => response.url().includes('/static/css/app.css')
  );
  const jsPromise = page.waitForResponse(
    response => response.url().includes('/static/js/game.js')
  );

  await page.goto('/');

  // Wait for responses
  const cssResponse = await cssPromise;
  expect(cssResponse.status()).toBe(200);
  expect(cssResponse.headers()['content-type']).toContain('text/css');

  const jsResponse = await jsPromise;
  expect(jsResponse.status()).toBe(200);
  expect(jsResponse.headers()['content-type']).toContain('javascript');
});

/**
 * Test: Health check endpoint
 */
test('health check', async ({ request }) => {
  const response = await request.get('/health');
  expect(response.ok()).toBeTruthy();
  expect(response.status()).toBe(200);
});
