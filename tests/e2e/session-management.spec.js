// @ts-check
const { test, expect } = require('@playwright/test');

/**
 * Test Suite: Session Management E2E Tests
 *
 * This suite validates session lifecycle management:
 * - Session creation and ID retrieval
 * - Session state retrieval via API
 * - Multiple concurrent sessions
 * - Session persistence across page reloads
 * - 404 handling for non-existent sessions
 *
 * Requirements: 4.3, 5.1, 5.2, 6.4
 */

/**
 * Test: Session creation returns valid UUID v4 session_id
 */
test('session creation returns valid session_id', async ({ page, request }) => {
  // Navigate to lobby
  await page.goto('/');
  await page.waitForSelector('button:has-text("START GAME")', { timeout: 5000 });

  // Intercept session creation request
  const responsePromise = page.waitForResponse(
    response => response.url().includes('/api/sessions') && response.status() === 201
  );

  // Create session
  await page.click('button:has-text("START GAME")');

  // Wait for response
  const response = await responsePromise;
  expect(response.status()).toBe(201);

  // Wait for poker table to load (game successfully started)
  await page.waitForSelector('.poker-table', { timeout: 10000 });

  // Extract session_id from the HTML response body
  const responseText = await response.text();
  const sessionIdMatch = responseText.match(/data-session-id="([0-9a-f-]+)"/i);
  expect(sessionIdMatch).toBeTruthy();
  const sessionId = sessionIdMatch[1];

  // Validate UUID v4 format (8-4-4-4-12 hex digits)
  const uuidV4Regex = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
  expect(sessionId).toMatch(uuidV4Regex);

  // Verify session_id is accessible via data attribute
  expect(sessionId).toBeTruthy();
  expect(sessionId.length).toBe(36); // UUID v4 length with hyphens
});

/**
 * Test: GET /api/sessions/{id}/state returns correct GameStateResponse structure
 */
test('session state retrieval via API', async ({ page, request }) => {
  // Create a session first
  await page.goto('/');
  await page.waitForSelector('button:has-text("START GAME")', { timeout: 5000 });

  // Intercept response to extract session_id
  const responsePromise = page.waitForResponse(
    response => response.url().includes('/api/sessions') && response.status() === 201
  );

  await page.click('button:has-text("START GAME")');

  // Wait for response and extract session_id
  const response = await responsePromise;
  await page.waitForSelector('.poker-table', { timeout: 10000 });

  const responseText = await response.text();
  const sessionIdMatch = responseText.match(/data-session-id="([0-9a-f-]+)"/i);
  expect(sessionIdMatch).toBeTruthy();
  const sessionId = sessionIdMatch[1];

  // Call GET /api/sessions/{id}/state
  const stateResponse = await request.get(`/api/sessions/${sessionId}/state`);
  expect(stateResponse.ok()).toBeTruthy();
  expect(stateResponse.status()).toBe(200);

  // Verify JSON response structure
  const state = await stateResponse.json();

  // Validate required fields per design.md GameStateResponse schema
  expect(state).toHaveProperty('session_id');
  expect(state.session_id).toBe(sessionId);

  expect(state).toHaveProperty('players');
  expect(Array.isArray(state.players)).toBeTruthy();
  expect(state.players.length).toBe(2); // Head-to-head poker

  expect(state).toHaveProperty('board');
  expect(Array.isArray(state.board)).toBeTruthy();

  expect(state).toHaveProperty('pot');
  expect(typeof state.pot).toBe('number');
  expect(state.pot).toBeGreaterThanOrEqual(0);

  expect(state).toHaveProperty('current_player');
  expect(typeof state.current_player).toBe('number');

  expect(state).toHaveProperty('available_actions');
  expect(Array.isArray(state.available_actions)).toBeTruthy();

  expect(state).toHaveProperty('hand_id');
  expect(typeof state.hand_id).toBe('string');

  expect(state).toHaveProperty('street');
  expect(typeof state.street).toBe('string');

  // Validate player structure
  const player0 = state.players[0];
  expect(player0).toHaveProperty('id');
  expect(player0).toHaveProperty('stack');
  expect(player0).toHaveProperty('position');
  expect(player0).toHaveProperty('is_active');

  // Player 0 should have hole_cards (human player)
  if (player0.id === 0) {
    expect(player0).toHaveProperty('hole_cards');
    expect(Array.isArray(player0.hole_cards)).toBeTruthy();
  }
});

/**
 * Test: Multiple concurrent sessions maintain independent state
 */
test('concurrent sessions are independent', async ({ browser }) => {
  // Create two separate browser contexts (simulating two different users)
  const context1 = await browser.newContext();
  const context2 = await browser.newContext();

  const page1 = await context1.newPage();
  const page2 = await context2.newPage();

  try {
    // Session 1: Create first session
    await page1.goto('/');
    await page1.waitForSelector('button:has-text("START GAME")', { timeout: 5000 });
    const response1Promise = page1.waitForResponse(
      response => response.url().includes('/api/sessions') && response.status() === 201
    );
    await page1.click('button:has-text("START GAME")');
    const response1 = await response1Promise;
    await page1.waitForSelector('.poker-table', { timeout: 10000 });
    const responseText1 = await response1.text();
    const sessionIdMatch1 = responseText1.match(/data-session-id="([0-9a-f-]+)"/i);
    expect(sessionIdMatch1).toBeTruthy();
    const sessionId1 = sessionIdMatch1[1];

    // Session 2: Create second session
    await page2.goto('/');
    await page2.waitForSelector('button:has-text("START GAME")', { timeout: 5000 });
    const response2Promise = page2.waitForResponse(
      response => response.url().includes('/api/sessions') && response.status() === 201
    );
    await page2.click('button:has-text("START GAME")');
    const response2 = await response2Promise;
    await page2.waitForSelector('.poker-table', { timeout: 10000 });
    const responseText2 = await response2.text();
    const sessionIdMatch2 = responseText2.match(/data-session-id="([0-9a-f-]+)"/i);
    expect(sessionIdMatch2).toBeTruthy();
    const sessionId2 = sessionIdMatch2[1];

    // Verify different session IDs
    expect(sessionId1).toBeTruthy();
    expect(sessionId2).toBeTruthy();
    expect(sessionId1).not.toBe(sessionId2);

    // Verify both sessions have independent game states
    const apiContext = await browser.newContext();
    const apiPage = await apiContext.newPage();

    const state1Response = await apiPage.request.get(`http://127.0.0.1:8080/api/sessions/${sessionId1}/state`);
    const state2Response = await apiPage.request.get(`http://127.0.0.1:8080/api/sessions/${sessionId2}/state`);

    expect(state1Response.ok()).toBeTruthy();
    expect(state2Response.ok()).toBeTruthy();

    const state1 = await state1Response.json();
    const state2 = await state2Response.json();

    // Verify session IDs match
    expect(state1.session_id).toBe(sessionId1);
    expect(state2.session_id).toBe(sessionId2);

    // Verify independent state (different hand numbers or player positions possible)
    // Even with same seed, sessions should maintain separate state
    expect(state1).not.toEqual(state2);

    await apiContext.close();
  } finally {
    await context1.close();
    await context2.close();
  }
});

/**
 * Test: Session persistence - session_id survives page reload
 */
test('session persists after page reload', async ({ page, request }) => {
  // Create a session
  await page.goto('/');
  await page.waitForSelector('button:has-text("START GAME")', { timeout: 5000 });

  const responsePromise = page.waitForResponse(
    response => response.url().includes('/api/sessions') && response.status() === 201
  );

  await page.click('button:has-text("START GAME")');

  const response = await responsePromise;
  await page.waitForSelector('.poker-table', { timeout: 10000 });

  const responseText = await response.text();
  const sessionIdMatch = responseText.match(/data-session-id="([0-9a-f-]+)"/i);
  expect(sessionIdMatch).toBeTruthy();
  const originalSessionId = sessionIdMatch[1];

  // Get initial state
  const initialStateResponse = await request.get(`/api/sessions/${originalSessionId}/state`);
  expect(initialStateResponse.ok()).toBeTruthy();
  const initialState = await initialStateResponse.json();
  const initialHandNumber = initialState.hand_number;

  // Reload the page
  await page.reload();

  // Note: After reload, we should see the lobby again (session ID is in backend, not persisted in browser)
  // This tests that the session still exists on the server side
  const laterStateResponse = await request.get(`/api/sessions/${originalSessionId}/state`);
  expect(laterStateResponse.ok()).toBeTruthy();

  const laterState = await laterStateResponse.json();

  // Verify session still exists and has same session_id
  expect(laterState.session_id).toBe(originalSessionId);

  // State should be preserved (same hand_number since no actions taken)
  expect(laterState.hand_number).toBe(initialHandNumber);

  // Verify session structure is still valid
  expect(laterState).toHaveProperty('players');
  expect(laterState).toHaveProperty('pot');
  expect(laterState).toHaveProperty('board');
});

/**
 * Test: Non-existent session returns 404
 */
test('non-existent session returns 404', async ({ request }) => {
  // Generate a valid UUID v4 format that doesn't exist
  const fakeSessionId = '00000000-0000-4000-8000-000000000000';

  // Attempt to get state of non-existent session
  const response = await request.get(`/api/sessions/${fakeSessionId}/state`);

  // Expect 404 Not Found
  expect(response.status()).toBe(404);

  // Verify error response format
  const errorBody = await response.json();
  expect(errorBody).toHaveProperty('error');

  // Error code should indicate session not found (matching Rust error code format)
  expect(errorBody.error).toBe('session_not_found');
});

/**
 * Test: GET /api/sessions/{id} returns full SessionResponse with config
 */
test('get full session info with config', async ({ page, request }) => {
  // Create a session with specific config
  await page.goto('/');
  await page.waitForSelector('button:has-text("START GAME")', { timeout: 5000 });

  // Select level 2
  await page.selectOption('select#level', '2');
  await page.selectOption('select#opponent_type', 'ai:aggressive');

  const responsePromise = page.waitForResponse(
    response => response.url().includes('/api/sessions') && response.status() === 201
  );

  await page.click('button:has-text("START GAME")');

  const response = await responsePromise;
  await page.waitForSelector('.poker-table', { timeout: 10000 });

  const responseText = await response.text();
  const sessionIdMatch = responseText.match(/data-session-id="([0-9a-f-]+)"/i);
  expect(sessionIdMatch).toBeTruthy();
  const sessionId = sessionIdMatch[1];

  // Call GET /api/sessions/{id} (full session info)
  const sessionResponse = await request.get(`/api/sessions/${sessionId}`);
  expect(sessionResponse.ok()).toBeTruthy();
  expect(sessionResponse.status()).toBe(200);

  const session = await sessionResponse.json();

  // Verify SessionResponse structure per design.md
  expect(session).toHaveProperty('session_id');
  expect(session.session_id).toBe(sessionId);

  expect(session).toHaveProperty('config');
  expect(session.config).toHaveProperty('level');
  expect(session.config.level).toBe(2); // We selected level 2

  expect(session.config).toHaveProperty('opponent_type');
  expect(session.config.opponent_type).toBe('ai:aggressive');

  expect(session).toHaveProperty('state');
  expect(session.state).toHaveProperty('session_id');
  expect(session.state.session_id).toBe(sessionId);
});

/**
 * Test: Invalid session ID format returns error
 */
test('invalid session ID format returns error', async ({ request }) => {
  // Use an invalid format (not UUID)
  const invalidSessionId = 'not-a-valid-uuid';

  // Attempt to get state
  const response = await request.get(`/api/sessions/${invalidSessionId}/state`);

  // Expect error (could be 400 Bad Request or 404 Not Found depending on implementation)
  expect(response.ok()).toBeFalsy();
  expect([400, 404]).toContain(response.status());
});
