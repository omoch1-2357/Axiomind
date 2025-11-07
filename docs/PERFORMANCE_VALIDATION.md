# Performance Validation Guide

This document provides manual validation procedures for rustdoc documentation performance metrics that cannot be automated in integration tests.

## Overview

The rustdoc automation feature includes performance requirements:
1. **CI Build Time**: cargo doc --workspace --no-deps should complete in 5 minutes or less
2. **Cargo Cache Effectiveness**: Second build should be significantly faster (2x+ speedup)
3. **GitHub Pages Load Time**: Top page should load in 1 second or less
4. **Lighthouse Performance Score**: Documentation site should score 90+ in performance

Automated tests cover items 1-2. This guide covers manual validation for items 3-4.

## Automated Performance Tests

Run automated performance tests:

```bash
# Run all performance tests
cargo test --test test_performance_metrics

# Run specific test
cargo test --test test_performance_metrics test_cargo_doc_build_time
cargo test --test test_performance_metrics test_cargo_cache_effectiveness
```

Test output includes:
- Build time measurements
- Cache speedup ratios
- Documentation size metrics
- Workflow execution time

## Manual Performance Validation

### 1. GitHub Pages Load Time Measurement

**Objective**: Verify that the documentation homepage loads in 1 second or less.

**Prerequisites**:
- Documentation is deployed to GitHub Pages
- You have a web browser with DevTools (Chrome, Firefox, Edge)

**Procedure**:

1. Open GitHub Pages URL in incognito/private mode (to avoid cache):
   ```
   https://<username>.github.io/Axiomind/
   ```

2. Open DevTools (F12 or Right-click → Inspect)

3. Go to the Network tab

4. Ensure "Disable cache" is checked

5. Reload the page (Ctrl+R or Cmd+R)

6. Check the timing metrics at the bottom:
   - **DOMContentLoaded**: Should be < 500ms
   - **Load**: Should be < 1000ms (1 second)

7. Record the results:
   ```
   Date: YYYY-MM-DD
   Browser: Chrome/Firefox/Edge version X
   DOMContentLoaded: XXXms
   Load: XXXms
   Status: PASS/FAIL
   ```

**Success Criteria**:
- Load time ≤ 1000ms
- DOMContentLoaded ≤ 500ms
- No console errors

**Troubleshooting**:
- If load time > 1s, check file sizes in Network tab
- Look for large assets (images, JavaScript files)
- Verify GitHub Pages CDN is being used (check response headers)

### 2. Lighthouse Performance Audit

**Objective**: Verify documentation site scores 90+ in Lighthouse performance audit.

**Prerequisites**:
- Documentation is deployed to GitHub Pages
- Chrome browser (Lighthouse is built-in)

**Procedure**:

1. Open GitHub Pages URL:
   ```
   https://<username>.github.io/Axiomind/
   ```

2. Open DevTools (F12)

3. Go to the Lighthouse tab
   - If not visible, click the >> icon and select Lighthouse

4. Configure audit:
   - **Mode**: Navigation
   - **Categories**: Performance only (uncheck others for faster audit)
   - **Device**: Desktop

5. Click "Analyze page load"

6. Wait for audit to complete (30-60 seconds)

7. Review results:
   - **Performance Score**: Target 90+
   - **First Contentful Paint (FCP)**: < 1.8s
   - **Largest Contentful Paint (LCP)**: < 2.5s
   - **Total Blocking Time (TBT)**: < 200ms
   - **Cumulative Layout Shift (CLS)**: < 0.1
   - **Speed Index**: < 3.4s

8. Record the results:
   ```
   Date: YYYY-MM-DD
   Performance Score: XX/100
   FCP: XXXms
   LCP: XXXms
   TBT: XXXms
   CLS: X.XX
   Speed Index: XXXms
   Status: PASS/FAIL
   ```

**Success Criteria**:
- Performance Score ≥ 90
- All Core Web Vitals in "good" range

**Troubleshooting**:
- **Low score**: Click on failing metrics to see recommendations
- **Large JavaScript**: Check for unnecessary dependencies
- **Slow server response**: Verify GitHub Pages CDN is active
- **Render-blocking resources**: Check if CSS/JS is blocking rendering

### 3. Cross-Browser Performance Testing

**Objective**: Verify documentation loads quickly across multiple browsers.

**Prerequisites**:
- Access to Chrome, Firefox, and Safari/Edge
- Documentation deployed to GitHub Pages

**Procedure**:

For each browser (Chrome, Firefox, Safari/Edge):

1. Open browser in incognito/private mode

2. Navigate to GitHub Pages URL

3. Open DevTools Network tab

4. Disable cache

5. Reload page and measure load time

6. Record results:
   ```
   Browser: Chrome/Firefox/Safari/Edge
   Version: X.Y.Z
   Load Time: XXXms
   Status: PASS/FAIL
   ```

**Success Criteria**:
- All browsers load in < 1s
- No browser-specific errors

### 4. Mobile Performance Testing

**Objective**: Verify documentation is performant on mobile devices.

**Prerequisites**:
- Chrome DevTools with mobile emulation
- Or physical mobile device

**Procedure (Desktop - Mobile Emulation)**:

1. Open GitHub Pages URL in Chrome

2. Open DevTools (F12)

3. Click device toolbar icon (Ctrl+Shift+M or Cmd+Shift+M)

4. Select device:
   - Pixel 5 (Android)
   - iPhone 12 Pro (iOS)

5. Ensure throttling is enabled:
   - **Network**: Fast 3G
   - **CPU**: 4x slowdown

6. Reload page

7. Run Lighthouse audit with Mobile mode

8. Record results:
   ```
   Device: Pixel 5 / iPhone 12 Pro
   Performance Score: XX/100
   Load Time: XXXms
   Status: PASS/FAIL
   ```

**Success Criteria**:
- Mobile Performance Score ≥ 80
- Load time < 3s on throttled connection

## Performance Benchmarks

### Expected Results (Reference Values)

Based on current implementation:

| Metric | Target | Typical | Max Acceptable |
|--------|--------|---------|----------------|
| **CI Build Time** | < 5 min | 2-3 min | 6 min |
| **Cache Speedup** | > 2x | 5-10x | 2x |
| **Page Load Time** | < 1s | 500-800ms | 1s |
| **Lighthouse Score** | > 90 | 95-100 | 90 |
| **FCP** | < 1.8s | 300-600ms | 1.8s |
| **LCP** | < 2.5s | 800-1200ms | 2.5s |
| **Doc Size** | < 100MB | 10-20MB | 100MB |

### Historical Performance Data

Record performance measurements over time to track trends:

```
Date       | Build Time | Cache Speedup | Load Time | Lighthouse
-----------|------------|---------------|-----------|------------
2025-11-06 | 2m 45s     | 8.5x          | 650ms     | 98
YYYY-MM-DD | ...        | ...           | ...       | ...
```

## Automated CI Performance Checks

The CI pipeline includes automated performance checks in the `rustdoc` job:

```yaml
# .github/workflows/ci.yml
- name: Build documentation
  run: |
    echo "Starting documentation build..."
    time cargo doc --workspace --no-deps --verbose
```

Monitor CI logs for:
- Build time trends
- Cache hit/miss rates
- Warning messages about performance

## Performance Optimization Tips

If performance metrics fall below targets:

### Build Time Optimization
- Verify Cargo cache is configured correctly in CI
- Check for unnecessary dependencies
- Use `--no-deps` flag to exclude external crates
- Consider incremental compilation settings

### Page Load Optimization
- Minimize custom CSS/JavaScript
- Use GitHub Pages CDN effectively
- Optimize images (if any)
- Enable browser caching headers

### Documentation Size Optimization
- Remove `--document-private-items` flag (already excluded)
- Verify external dependencies are excluded with `--no-deps`
- Consider compressing large static assets

## Reporting Performance Issues

If performance metrics consistently fail:

1. **Document the issue**:
   - Which metric is failing
   - Measured value vs. target
   - Browser/environment details
   - Screenshots of Lighthouse report

2. **Create GitHub issue**:
   - Label: `performance`, `documentation`
   - Include benchmark data
   - Attach Lighthouse report JSON

3. **Investigation steps**:
   - Check recent changes to documentation
   - Review CI build logs
   - Compare against historical benchmarks
   - Test in multiple environments

## Continuous Monitoring

Set up periodic performance checks:

- **Weekly**: Run automated performance tests locally
- **Monthly**: Perform manual Lighthouse audits
- **After major changes**: Full performance validation
- **Before releases**: Complete performance benchmark

---

**Last Updated**: 2025-11-06
**Document Version**: 1.0
