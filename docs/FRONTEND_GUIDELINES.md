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

**Problem**: `hx-ext="json-enc"` doesn't work without loading the extension.

#### ✅ CORRECT - Manual JSON submission
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

  // Handle response...
});
</script>
```

#### ✅ ALTERNATIVE - Use htmx with proper extension
```html
<script src="https://unpkg.com/htmx.org@1.9.12"></script>
<script src="https://unpkg.com/htmx.org@1.9.12/dist/ext/json-enc.js"></script>

<form hx-post="/api/sessions" hx-ext="json-enc">
  <!-- Now sends application/json -->
  <input name="level" value="1" />
</form>
```

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
```javascript
// tests/e2e/game-start.spec.js
import { test, expect } from '@playwright/test';

test('start game with correct API format', async ({ page }) => {
  await page.goto('http://localhost:8080');

  // Intercept API request
  const requestPromise = page.waitForRequest(request =>
    request.url().includes('/api/sessions') &&
    request.method() === 'POST'
  );

  await page.click('button:has-text("START GAME")');

  const request = await requestPromise;

  // Verify Content-Type
  expect(request.headers()['content-type']).toBe('application/json');

  // Verify payload format
  const body = JSON.parse(request.postData());
  expect(body).toHaveProperty('level');
  expect(body).toHaveProperty('opponent_type');
  expect(body.opponent_type).toMatch(/^ai:/);
});
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
