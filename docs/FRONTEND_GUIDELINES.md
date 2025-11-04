# Frontend Development Guidelines

**Target Audience**: AI assistants (Claude Code) and human developers working on `rust/web/static/`

## Lessons Learned: rust-web-server Incident (2025-11-02)

### What Went Wrong
1. **JavaScript syntax error** - Template literal misuse in `game.js:66-69`
2. **htmx misconfiguration** - Content-Type mismatch (`application/x-www-form-urlencoded` vs `application/json`)
3. **No static analysis** - ESLint not configured
4. **No browser testing** - Playwright not used
5. **False confidence** - 178 Rust tests passed, but UI didn't work

### The Rule
> **Passing backend tests does NOT mean the frontend works.**
> **You MUST test in a real browser.**

---

## Mandatory Checklist for Frontend Changes

Before committing ANY changes to `rust/web/static/`:

- [ ] ✅ Run `npm run lint` - ESLint passes
- [ ] ✅ Run `node --check <file>` - No syntax errors
- [ ] ✅ Run `npm run test:e2e` - Playwright tests pass
- [ ] ✅ **Open browser and test manually** - Visual verification
- [ ] ✅ Check console (F12) - No JavaScript errors
- [ ] ✅ Verify Network tab - Correct Content-Type headers
- [ ] ✅ Test form submissions - Data sent in expected format

**If you skip any step, the app WILL break in production.**

---

## JavaScript Coding Standards

### 1. Template Literals

```javascript
// ❌ WRONG - Causes syntax error
const html = `
  ${condition ? value : '
    <div>content</div>
  '}
`;

// ✅ CORRECT - Use backticks consistently
const html = `
  ${condition ? value : `
    <div>content</div>
  `}
`;
```

### 2. htmx Integration

#### ❌ WRONG - Content-Type mismatch
```html
<form hx-post="/api/sessions" hx-ext="json-enc">
  <!-- This sends application/x-www-form-urlencoded -->
  <input name="level" value="1" />
</form>
```

**Problem**: `hx-ext="json-enc"` doesn't work without loading the extension script.

#### ✅ CORRECT - Use htmx with json-enc extension
```html
<!-- Load htmx and json-enc extension in <head> -->
<script src="https://unpkg.com/htmx.org@1.9.12"></script>
<script src="https://unpkg.com/htmx.org@1.9.12/dist/ext/json-enc.js"></script>

<!-- Form now sends application/json -->
<form hx-post="/api/sessions" hx-ext="json-enc" hx-target="#table" hx-swap="innerHTML">
  <input name="level" value="1" />
  <input name="opponent_type" value="ai:baseline" />
  <button type="submit">START GAME</button>
</form>
```

**How it works**:
1. `json-enc.js` intercepts form submissions marked with `hx-ext="json-enc"`
2. Converts FormData to JSON: `{"level": "1", "opponent_type": "ai:baseline"}`
3. Sets `Content-Type: application/json` header automatically
4. Server receives proper JSON payload that can be deserialized

**Common htmx attributes**:
- `hx-post="/api/sessions"` - Endpoint to POST to
- `hx-ext="json-enc"` - Enable JSON encoding extension
- `hx-target="#table"` - Where to insert response HTML
- `hx-swap="innerHTML"` - How to insert (innerHTML, outerHTML, beforebegin, afterend, etc.)
- `hx-vals='js:{...}'` - Add extra data (for dynamic values from JavaScript)

#### ✅ ALTERNATIVE - Manual JSON submission with Fetch API
```html
<form id="start-game-form">
  <input name="level" value="1" />
  <button type="submit">Start</button>
</form>

<script>
document.getElementById('start-game-form').addEventListener('submit', async (e) => {
  e.preventDefault();
  const formData = new FormData(e.target);
  const data = Object.fromEntries(formData);

  const response = await fetch('/api/sessions', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(data)
  });

  if (response.ok) {
    const html = await response.text();
    document.getElementById('table').innerHTML = html;
  }
});
</script>
```

**When to use Fetch instead of htmx**:
- Complex client-side validation required before submission
- Need to manipulate response before rendering
- Want to show loading spinners or progress indicators
- Need more control over error handling

### 3. API Data Formats

**Server expects**:
```json
{
  "level": 1,
  "opponent_type": "ai:baseline"
}
```

**NOT**:
- `"opponent_type": "AI"` ❌
- `"opponent_type": {"AI": "baseline"}` ❌
- Form data: `level=1&opponent_type=ai:baseline` ❌

**Validation**:
```javascript
// Always verify before sending
const payload = {
  level: parseInt(formData.get('level')),
  opponent_type: formData.get('opponent_type')
};

console.log('Sending:', JSON.stringify(payload));

const response = await fetch('/api/sessions', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify(payload)
});

if (!response.ok) {
  const error = await response.text();
  console.error('API Error:', error);
}
```

---

## Testing Requirements

### 1. Unit Tests (Optional for Simple Scripts)
```javascript
// Only if complex logic exists
export function calculateBetAmount(pot, percentage) {
  return Math.floor(pot * percentage / 100);
}

// Test with Jest/Vitest
test('calculates bet correctly', () => {
  expect(calculateBetAmount(100, 50)).toBe(50);
});
```

### 2. E2E Tests (MANDATORY for All Features)

**Why E2E tests are non-negotiable**:
- Rust tests validate backend logic, NOT browser behavior
- JavaScript can have syntax errors that don't show up in Rust tests
- htmx integration requires real browser testing
- Content-Type headers must be verified in actual HTTP requests

**E2E Test Structure** (following web-server-fixes spec):
```
tests/e2e/
├── game-flow.spec.js          # Critical: game start, user actions
├── sse-events.spec.js          # SSE connection, event handling
├── error-handling.spec.js      # API errors, console errors
├── betting-controls.spec.js    # Form validation, user input
├── session-management.spec.js  # Session lifecycle
├── edge-cases.spec.js          # Showdown, split pot, edge cases
└── static-assets.spec.js       # CSS/JS loading, Content-Type
```

**Example: Complete E2E test with all validations**
```javascript
// tests/e2e/game-start.spec.js
import { test, expect } from '@playwright/test';

test('start game with correct API format', async ({ page }) => {
  // 1. Capture console errors
  const consoleErrors = [];
  page.on('console', msg => {
    if (msg.type() === 'error') {
      consoleErrors.push(msg.text());
    }
  });

  // 2. Navigate to application
  await page.goto('http://localhost:8080');

  // 3. Verify lobby loads
  await expect(page.locator('h2')).toContainText('Game Lobby');

  // 4. Intercept API request to verify Content-Type and payload
  const requestPromise = page.waitForRequest(request =>
    request.url().includes('/api/sessions') &&
    request.method() === 'POST'
  );

  // 5. Trigger action
  await page.click('button:has-text("START GAME")');

  // 6. Verify request details
  const request = await requestPromise;

  // Content-Type validation (critical!)
  expect(request.headers()['content-type']).toBe('application/json');

  // Payload structure validation
  const body = JSON.parse(request.postData());
  expect(body).toHaveProperty('level');
  expect(body).toHaveProperty('opponent_type');
  expect(body.opponent_type).toMatch(/^(ai:[\w]+|human)$/);

  // 7. Verify response and UI update
  const response = await page.waitForResponse('/api/sessions');
  expect(response.status()).toBe(201);

  // 8. Check DOM updates
  await expect(page.locator('.poker-table')).toBeVisible();
  await expect(page.locator('.hole-cards')).toBeVisible();

  // 9. Ensure no JavaScript errors occurred
  expect(consoleErrors).toHaveLength(0);
});
```

**E2E Testing Checklist** (add this to every new feature):
- [ ] Test navigates to the page
- [ ] Intercepts API requests to verify Content-Type
- [ ] Validates request payload structure
- [ ] Checks response status codes (201, 200, 404, etc.)
- [ ] Verifies DOM updates after actions
- [ ] Monitors console for JavaScript errors
- [ ] Tests error scenarios (invalid input, API failures)
- [ ] Includes screenshots/traces on failure

**Common E2E Patterns**:

```javascript
// Wait for selector before interacting
await page.waitForSelector('button:has-text("START GAME")');
await page.click('button:has-text("START GAME")');

// Verify element text content
await expect(page.locator('.pot-display')).toHaveText('Pot: 150');

// Fill form inputs
await page.fill('input[name="bet-amount"]', '100');

// Check element visibility
await expect(page.locator('.error-message')).not.toBeVisible();

// Intercept and inspect network requests
page.on('request', req => console.log('>>', req.method(), req.url()));
page.on('response', res => console.log('<<', res.status(), res.url()));

// Wait for network idle (useful after SSE connections)
await page.waitForLoadState('networkidle');
```

### 3. Manual Browser Testing
1. Open http://localhost:8080
2. Open DevTools (F12)
3. Click through all UI flows
4. Check Console tab - Should be NO errors
5. Check Network tab - Verify request/response formats
6. Test edge cases (empty forms, invalid input)

---

## Common Pitfalls

### Pitfall 1: Assuming Tests Cover Frontend
```
❌ "Rust tests passed, so the frontend works"
✅ "I tested in a browser and verified the UI"
```

### Pitfall 2: Ignoring JavaScript Errors
```
❌ "There's a console error, but the page loads"
✅ "I fixed all console errors before committing"
```

### Pitfall 3: Not Checking Content-Type
```
❌ "The API returns 415, but I don't know why"
✅ "I verified Content-Type matches server expectations"
```

### Pitfall 4: Mixing Quote Types in Templates
```javascript
❌ `${condition ? value : ' ... '}`  // Syntax error
✅ `${condition ? value : ` ... `}`  // Correct
```

---

## File Structure

```
rust/web/static/
├── index.html           # Entry point
├── css/
│   └── app.css         # Styles
├── js/
│   ├── htmx.min.js     # htmx library (vendored)
│   └── game.js         # Game UI logic
└── img/                # Images (if any)
```

---

## ESLint Configuration

```javascript
// .eslintrc.json
{
  "env": {
    "browser": true,
    "es2021": true
  },
  "extends": "eslint:recommended",
  "parserOptions": {
    "ecmaVersion": "latest",
    "sourceType": "module"
  },
  "rules": {
    "no-unused-vars": "warn",
    "no-console": "off",
    "quotes": ["error", "single"],
    "semi": ["error", "always"]
  }
}
```

---

## Browser Compatibility

**Target**: Modern browsers (Chrome/Firefox/Safari last 2 versions)

**DO NOT use**:
- Internet Explorer specific code
- Experimental APIs without fallbacks
- Vendor prefixes without autoprefixer

**DO use**:
- Modern JavaScript (ES2021+)
- CSS Grid and Flexbox
- Fetch API (not XMLHttpRequest)

---

## Debugging Workflow

### Issue: "The button doesn't work"

1. **Check console**
   ```
   F12 → Console tab
   Look for red errors
   ```

2. **Check network**
   ```
   F12 → Network tab → Click button
   Verify request sent
   Check status code
   Inspect request/response
   ```

3. **Check htmx**
   ```html
   <div hx-boost="true">
   <!-- Enable htmx logging -->
   <script>
   htmx.logger = function(elt, event, data) {
     console.log(event, data);
   }
   </script>
   ```

4. **Check server logs**
   ```bash
   tail -f web_server.log
   ```

---

## Workflow: Adding New JavaScript Features

**Follow this order** (Test-Driven Development):

1. **Write E2E test first** (RED phase)
   ```bash
   # Create test file
   touch tests/e2e/my-new-feature.spec.js
   # Write test that describes expected behavior
   # Run test (it should FAIL)
   npm run test:e2e
   ```

2. **Implement feature** (GREEN phase)
   ```javascript
   // Add JavaScript code in rust/web/static/js/game.js
   function myNewFeature() {
     // Implementation
   }
   ```

3. **Run ESLint** (VERIFY phase)
   ```bash
   npm run lint
   # Fix any errors
   npm run lint:fix
   ```

4. **Run E2E test again**
   ```bash
   npm run test:e2e
   # Should PASS now
   ```

5. **Manual browser test**
   ```bash
   cargo run --release -p axm_web
   # Open http://localhost:8080
   # Click through UI, check console (F12)
   ```

6. **Commit**
   ```bash
   git add .
   git commit -m "feat: add my new feature"
   # Pre-commit hook runs automatically
   ```

**DO NOT skip steps 3-5.** Rust tests alone are NOT enough.

## Pre-Commit Checklist

Run this command before every commit:

```bash
# Full validation
npm run lint &&
npm run test:e2e &&
cargo test --workspace
```

If any step fails, **DO NOT COMMIT**.

---

## References

- [htmx Documentation](https://htmx.org/docs/)
- [Playwright Best Practices](https://playwright.dev/docs/best-practices)
- [MDN Web Docs](https://developer.mozilla.org/)
- Incident Report: rust-web-server (2025-11-02) - See `docs/TESTING.md`
