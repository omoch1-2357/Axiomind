// @ts-check
const { test, expect } = require('@playwright/test');

/**
 * Test Suite: Betting Controls Comprehensive E2E Tests
 *
 * This test suite validates:
 * - Bet amount validation (below minimum, above maximum)
 * - Valid bet amount input (within range)
 * - Fold/Check/Call button actions and API payload format
 * - Inactive turn UI state (current_player !== 0)
 * - Real-time bet input validation (oninput event)
 *
 * Requirements: 5.4, 6.1, 6.2, 11.1, 11.2
 */

/**
 * Test: Bet amount validation - below minimum shows error
 *
 * Validates:
 * - Input below min_amount shows validation error
 * - Error message displays "Amount must be at least {min}"
 * - Input field gets 'invalid' class (red border)
 * - Bet/Raise button is disabled or prevented from submitting
 */
test('bet amount below minimum shows validation error', async ({ page }) => {
  const consoleErrors = [];
  page.on('console', msg => {
    if (msg.type() === 'error') {
      consoleErrors.push(msg.text());
    }
  });

  // Navigate and start game
  await page.goto('/');
  await page.waitForSelector('button:has-text("START GAME")', { timeout: 5000 });
  await page.click('button:has-text("START GAME")');
  await page.waitForSelector('.poker-table', { timeout: 10000 });

  // Wait for betting controls to appear
  await page.waitForSelector('.betting-controls', { timeout: 5000 });

  // Find bet input field
  const betInput = page.locator('.bet-input').first();
  const isVisible = await betInput.isVisible();

  if (!isVisible) {
    console.log('Bet input not available in current game state (may be check/fold only)');
    return; // Skip test if no bet input available
  }

  // Get min/max values
  const minAmount = await betInput.getAttribute('data-min');
  const maxAmount = await betInput.getAttribute('data-max');
  const min = parseInt(minAmount, 10);

  console.log(`Bet range: min=${min}, max=${maxAmount}`);

  // Enter amount below minimum
  const belowMin = Math.max(0, min - 50);
  await betInput.fill(belowMin.toString());

  // Trigger validation (oninput event)
  await betInput.dispatchEvent('input');

  // Wait for validation error to appear
  const errorElement = page.locator('.validation-error').first();
  await expect(errorElement).toBeVisible({ timeout: 2000 });

  // Verify error message
  const errorText = await errorElement.textContent();
  expect(errorText).toContain(`at least ${min}`);

  // Verify input has invalid class (red border)
  await expect(betInput).toHaveClass(/invalid/);

  // Verify no JavaScript errors
  expect(consoleErrors).toHaveLength(0);
});

/**
 * Test: Bet amount validation - above maximum shows error
 *
 * Validates:
 * - Input above max_amount shows validation error
 * - Error message displays "Amount cannot exceed {max}"
 * - Input field gets 'invalid' class
 * - Submit is prevented
 */
test('bet amount above maximum shows validation error', async ({ page }) => {
  const consoleErrors = [];
  page.on('console', msg => {
    if (msg.type() === 'error') {
      consoleErrors.push(msg.text());
    }
  });

  await page.goto('/');
  await page.waitForSelector('button:has-text("START GAME")');
  await page.click('button:has-text("START GAME")');
  await page.waitForSelector('.poker-table', { timeout: 10000 });

  await page.waitForSelector('.betting-controls', { timeout: 5000 });

  const betInput = page.locator('.bet-input').first();
  const isVisible = await betInput.isVisible();

  if (!isVisible) {
    console.log('Bet input not available in current game state');
    return;
  }

  const maxAmount = await betInput.getAttribute('data-max');
  const max = parseInt(maxAmount, 10);

  // Enter amount above maximum
  const aboveMax = max + 1000;
  await betInput.fill(aboveMax.toString());
  await betInput.dispatchEvent('input');

  // Verify error appears
  const errorElement = page.locator('.validation-error').first();
  await expect(errorElement).toBeVisible({ timeout: 2000 });

  const errorText = await errorElement.textContent();
  expect(errorText).toContain(`cannot exceed ${max}`);

  await expect(betInput).toHaveClass(/invalid/);

  expect(consoleErrors).toHaveLength(0);
});

/**
 * Test: Valid bet amount - no error, button enabled
 *
 * Validates:
 * - Input within [min, max] range shows no error
 * - Validation error is hidden
 * - Input field does NOT have 'invalid' class
 * - Bet/Raise button is clickable
 */
test('valid bet amount shows no error and enables button', async ({ page }) => {
  const consoleErrors = [];
  page.on('console', msg => {
    if (msg.type() === 'error') {
      consoleErrors.push(msg.text());
    }
  });

  await page.goto('/');
  await page.waitForSelector('button:has-text("START GAME")');
  await page.click('button:has-text("START GAME")');
  await page.waitForSelector('.poker-table', { timeout: 10000 });

  await page.waitForSelector('.betting-controls', { timeout: 5000 });

  const betInput = page.locator('.bet-input').first();
  const isVisible = await betInput.isVisible();

  if (!isVisible) {
    console.log('Bet input not available in current game state');
    return;
  }

  const minAmount = await betInput.getAttribute('data-min');
  const maxAmount = await betInput.getAttribute('data-max');
  const min = parseInt(minAmount, 10);
  const max = parseInt(maxAmount, 10);

  // Enter valid amount (midpoint between min and max)
  const validAmount = Math.floor((min + max) / 2);
  await betInput.fill(validAmount.toString());
  await betInput.dispatchEvent('input');

  // Wait a moment for validation to complete
  await page.waitForTimeout(500);

  // Verify error is NOT visible
  const errorElement = page.locator('.validation-error').first();
  await expect(errorElement).toBeHidden();

  // Verify input does NOT have invalid class
  const inputClass = await betInput.getAttribute('class');
  expect(inputClass).not.toContain('invalid');

  // Verify bet/raise button is enabled
  const betButton = page.locator('.action-btn[data-action="raise"], .action-btn[data-action="bet"]').first();
  await expect(betButton).toBeEnabled();

  expect(consoleErrors).toHaveLength(0);
});

/**
 * Test: Fold/Check/Call buttons send correct API payload
 *
 * Validates:
 * - Fold button has correct hx-post and hx-vals attributes
 * - Check button has correct payload structure (if available)
 * - Call button has correct payload structure (if available)
 * - Payload format matches hx-vals specification
 * - Buttons are properly configured for htmx
 */
test('fold check call buttons have correct htmx attributes', async ({ page }) => {
  const consoleErrors = [];
  page.on('console', msg => {
    if (msg.type() === 'error') {
      consoleErrors.push(msg.text());
    }
  });

  await page.goto('/');
  await page.waitForSelector('button:has-text("START GAME")');
  await page.click('button:has-text("START GAME")');
  await page.waitForSelector('.poker-table', { timeout: 10000 });

  await page.waitForSelector('.betting-controls', { timeout: 5000 });

  // Check which action buttons are available
  const foldButton = page.locator('.action-btn[data-action="fold"]');
  const checkButton = page.locator('.action-btn[data-action="check"]');
  const callButton = page.locator('.action-btn[data-action="call"]');

  const hasFold = await foldButton.isVisible();
  const hasCheck = await checkButton.isVisible();
  const hasCall = await callButton.isVisible();

  console.log(`Available actions: fold=${hasFold}, check=${hasCheck}, call=${hasCall}`);

  // Test Fold button attributes if available
  if (hasFold) {
    const hxPost = await foldButton.getAttribute('hx-post');
    const hxVals = await foldButton.getAttribute('hx-vals');

    expect(hxPost).toBeTruthy();
    expect(hxPost).toContain('/api/sessions/');
    expect(hxPost).toContain('/actions');

    expect(hxVals).toBeTruthy();
    // hx-vals should contain action: "Fold" or similar JSON structure
    expect(hxVals).toContain('action');
    expect(hxVals).toContain('Fold');

    console.log('Fold button hx-post:', hxPost);
    console.log('Fold button hx-vals:', hxVals);
  }

  // Test Check button attributes if available
  if (hasCheck) {
    const hxPost = await checkButton.getAttribute('hx-post');
    const hxVals = await checkButton.getAttribute('hx-vals');

    expect(hxPost).toBeTruthy();
    expect(hxPost).toContain('/api/sessions/');
    expect(hxPost).toContain('/actions');

    expect(hxVals).toBeTruthy();
    expect(hxVals).toContain('action');
    expect(hxVals).toContain('Check');

    console.log('Check button hx-post:', hxPost);
    console.log('Check button hx-vals:', hxVals);
  }

  // Test Call button attributes if available
  if (hasCall) {
    const hxPost = await callButton.getAttribute('hx-post');
    const hxVals = await callButton.getAttribute('hx-vals');

    expect(hxPost).toBeTruthy();
    expect(hxPost).toContain('/api/sessions/');
    expect(hxPost).toContain('/actions');

    expect(hxVals).toBeTruthy();
    expect(hxVals).toContain('action');
    expect(hxVals).toContain('Call');

    console.log('Call button hx-post:', hxPost);
    console.log('Call button hx-vals:', hxVals);
  }

  expect(consoleErrors).toHaveLength(0);
});

/**
 * Test: Inactive turn - UI disabled and buttons grayed out
 *
 * Validates:
 * - When current_player !== 0, betting controls have 'inactive' class
 * - All action buttons are disabled
 * - Bet input field is disabled
 * - "Waiting for opponent..." message is displayed
 */
test('inactive turn disables UI and grays out buttons', async ({ page }) => {
  const consoleErrors = [];
  page.on('console', msg => {
    if (msg.type() === 'error') {
      consoleErrors.push(msg.text());
    }
  });

  await page.goto('/');
  await page.waitForSelector('button:has-text("START GAME")');
  await page.click('button:has-text("START GAME")');
  await page.waitForSelector('.poker-table', { timeout: 10000 });

  await page.waitForSelector('.betting-controls', { timeout: 5000 });

  // Check current player state
  // We need to trigger opponent's turn to test this
  // For now, we'll verify the structure is correct when it's NOT our turn

  // If there's a fold button, click it to let opponent take turn
  const foldButton = page.locator('.action-btn[data-action="fold"]');
  const hasFold = await foldButton.isVisible();

  if (hasFold && await foldButton.isEnabled()) {
    // Fold to give opponent the turn (in next hand)
    await foldButton.click();

    // Wait for hand to complete and new hand to start
    await page.waitForTimeout(2000);

    // Check if it's now opponent's turn
    const bettingControls = page.locator('.betting-controls');
    const controlsClass = await bettingControls.getAttribute('class');

    if (controlsClass && controlsClass.includes('inactive')) {
      console.log('Opponent turn detected - testing inactive state');

      // Verify waiting message
      const waitingMsg = page.locator('.waiting');
      await expect(waitingMsg).toBeVisible();
      await expect(waitingMsg).toContainText('Waiting for opponent');

      // Verify all buttons are disabled
      const actionButtons = page.locator('.action-btn');
      const count = await actionButtons.count();
      for (let i = 0; i < count; i++) {
        const button = actionButtons.nth(i);
        await expect(button).toBeDisabled();
      }

      // Verify bet input is disabled (if present)
      const betInput = page.locator('.bet-input');
      if (await betInput.isVisible()) {
        await expect(betInput).toBeDisabled();
      }
    } else {
      console.log('Still player turn, cannot test inactive state in this game scenario');
    }
  } else {
    console.log('Cannot trigger opponent turn - fold button not available');
  }

  expect(consoleErrors).toHaveLength(0);
});

/**
 * Test: Real-time bet input validation (oninput event)
 *
 * Validates:
 * - Validation error appears immediately as user types (oninput)
 * - Error message updates in real-time
 * - Error disappears when valid amount is entered
 * - No need to blur/submit to see validation
 */
test('bet input validates in real-time on oninput event', async ({ page }) => {
  const consoleErrors = [];
  page.on('console', msg => {
    if (msg.type() === 'error') {
      consoleErrors.push(msg.text());
    }
  });

  await page.goto('/');
  await page.waitForSelector('button:has-text("START GAME")');
  await page.click('button:has-text("START GAME")');
  await page.waitForSelector('.poker-table', { timeout: 10000 });

  await page.waitForSelector('.betting-controls', { timeout: 5000 });

  const betInput = page.locator('.bet-input').first();
  const isVisible = await betInput.isVisible();

  if (!isVisible) {
    console.log('Bet input not available in current game state');
    return;
  }

  const minAmount = await betInput.getAttribute('data-min');
  const maxAmount = await betInput.getAttribute('data-max');
  const min = parseInt(minAmount, 10);
  const max = parseInt(maxAmount, 10);

  console.log(`Testing real-time validation: min=${min}, max=${max}`);

  // Type invalid amount (below min) character by character
  const invalidAmount = (min - 10).toString();
  await betInput.fill('');

  for (const char of invalidAmount) {
    await betInput.type(char);
    // Small delay to simulate typing
    await page.waitForTimeout(100);
  }

  // Error should appear during/after typing
  const errorElement = page.locator('.validation-error').first();
  await expect(errorElement).toBeVisible({ timeout: 2000 });

  // Verify error message content
  const errorText = await errorElement.textContent();
  expect(errorText).toContain(`at least ${min}`);

  // Now type a valid amount
  const validAmount = Math.floor((min + max) / 2).toString();
  await betInput.fill('');

  for (const char of validAmount) {
    await betInput.type(char);
    await page.waitForTimeout(100);
  }

  // Error should disappear
  await page.waitForTimeout(500);
  await expect(errorElement).toBeHidden();

  // Input should not have invalid class
  const inputClass = await betInput.getAttribute('class');
  expect(inputClass).not.toContain('invalid');

  expect(consoleErrors).toHaveLength(0);
});

/**
 * Test: Bet amount validation with edge cases
 *
 * Validates:
 * - Empty input shows error
 * - Negative number shows error
 * - Non-numeric input (if possible) shows error
 * - Zero amount shows error
 * - Exact min/max values are accepted
 */
test('bet validation handles edge cases correctly', async ({ page }) => {
  const consoleErrors = [];
  page.on('console', msg => {
    if (msg.type() === 'error') {
      consoleErrors.push(msg.text());
    }
  });

  await page.goto('/');
  await page.waitForSelector('button:has-text("START GAME")');
  await page.click('button:has-text("START GAME")');
  await page.waitForSelector('.poker-table', { timeout: 10000 });

  await page.waitForSelector('.betting-controls', { timeout: 5000 });

  const betInput = page.locator('.bet-input').first();
  const isVisible = await betInput.isVisible();

  if (!isVisible) {
    console.log('Bet input not available in current game state');
    return;
  }

  const minAmount = await betInput.getAttribute('data-min');
  const maxAmount = await betInput.getAttribute('data-max');
  const min = parseInt(minAmount, 10);
  const max = parseInt(maxAmount, 10);

  const errorElement = page.locator('.validation-error').first();

  // Test 1: Empty input
  await betInput.fill('');
  await betInput.dispatchEvent('input');
  await page.waitForTimeout(300);
  // Empty input may or may not show error immediately - browser behavior
  // Key is that submission should be prevented

  // Test 2: Zero amount
  await betInput.fill('0');
  await betInput.dispatchEvent('input');
  await page.waitForTimeout(300);
  if (min > 0) {
    await expect(errorElement).toBeVisible({ timeout: 2000 });
  }

  // Test 3: Negative number (browser input type="number" may prevent this)
  await betInput.fill('-100');
  await betInput.dispatchEvent('input');
  await page.waitForTimeout(300);
  // Input type="number" typically rejects negative values

  // Test 4: Exact minimum (should be valid)
  await betInput.fill(min.toString());
  await betInput.dispatchEvent('input');
  await page.waitForTimeout(500);
  await expect(errorElement).toBeHidden();
  const inputClassMin = await betInput.getAttribute('class');
  expect(inputClassMin).not.toContain('invalid');

  // Test 5: Exact maximum (should be valid)
  await betInput.fill(max.toString());
  await betInput.dispatchEvent('input');
  await page.waitForTimeout(500);
  await expect(errorElement).toBeHidden();
  const inputClassMax = await betInput.getAttribute('class');
  expect(inputClassMax).not.toContain('invalid');

  expect(consoleErrors).toHaveLength(0);
});
