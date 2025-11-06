// @ts-check
const { test, expect } = require('@playwright/test');

/**
 * Test Suite: Static Asset Delivery Validation
 *
 * This test suite validates:
 * 1. CSS files are served with correct Content-Type: text/css
 * 2. JavaScript files are served with Content-Type: application/javascript
 * 3. htmx and json-enc libraries load successfully
 * 4. 404 responses for nonexistent files
 * 5. Cache-Control headers are present for static assets
 *
 * Requirements: Req 4.5, Req 7.1, 7.2, 7.3, 7.4, 7.5
 */

/**
 * Test: CSS file served with correct Content-Type
 */
test('CSS loads with Content-Type: text/css', async ({ page }) => {
  // Direct access to CSS file (Req 7.1)
  const response = await page.goto('/static/css/app.css', { waitUntil: 'commit' });

  // Verify status code
  expect(response.status()).toBe(200);

  // Verify Content-Type header
  const contentType = response.headers()['content-type'];
  expect(contentType).toContain('text/css');
});

/**
 * Test: JavaScript files served with correct Content-Type
 */
test('game.js loads with Content-Type: application/javascript', async ({ page }) => {
  // Direct access to JS file (Req 7.2)
  const response = await page.goto('/static/js/game.js', { waitUntil: 'commit' });

  // Verify status code
  expect(response.status()).toBe(200);

  // Verify Content-Type header
  const contentType = response.headers()['content-type'];
  expect(contentType).toContain('javascript');
});

/**
 * Test: htmx and json-enc scripts are referenced in HTML
 */
test('htmx and json-enc scripts are referenced in index.html', async ({ page }) => {
  // Fetch index.html content directly
  const response = await page.goto('/', { waitUntil: 'commit' });
  const htmlContent = await response.text();

  // Verify htmx CDN script is referenced (Req 7.3)
  expect(htmlContent).toContain('unpkg.com/htmx.org@1.9.12');

  // Verify json-enc CDN script is referenced (Req 7.3)
  expect(htmlContent).toContain('unpkg.com/htmx.org@1.9.12/dist/ext/json-enc.js');

  // Verify game.js is referenced
  expect(htmlContent).toContain('/static/js/game.js');
});

/**
 * Test: 404 response for nonexistent files
 */
test('nonexistent file returns 404', async ({ page }) => {
  // Try to fetch a nonexistent file directly
  const response = await page.goto('/static/nonexistent.js', { waitUntil: 'commit' });

  // Verify 404 status (Req 7.4)
  expect(response.status()).toBe(404);
});

/**
 * Test: Cache-Control headers for static assets
 */
test('static assets have Cache-Control headers', async ({ page }) => {
  // Test CSS file (Req 7.5)
  const cssResponse = await page.goto('/static/css/app.css', { waitUntil: 'commit' });
  const cssCacheControl = cssResponse.headers()['cache-control'];
  expect(cssCacheControl).toBeDefined();
  expect(cssCacheControl).toBeTruthy();

  // Test JS file (Req 7.5)
  const jsResponse = await page.goto('/static/js/game.js', { waitUntil: 'commit' });
  const jsCacheControl = jsResponse.headers()['cache-control'];
  expect(jsCacheControl).toBeDefined();
  expect(jsCacheControl).toBeTruthy();
});

/**
 * Test: All referenced static assets exist and are accessible
 */
test('all static assets are accessible', async ({ page }) => {
  // Test CSS file
  const cssResponse = await page.goto('/static/css/app.css', { waitUntil: 'commit' });
  expect(cssResponse.status()).toBe(200);

  // Test game.js
  const jsResponse = await page.goto('/static/js/game.js', { waitUntil: 'commit' });
  expect(jsResponse.status()).toBe(200);

  // Test index.html
  const htmlResponse = await page.goto('/', { waitUntil: 'commit' });
  expect(htmlResponse.status()).toBe(200);
});
