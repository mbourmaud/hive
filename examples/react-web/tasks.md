# Task Templates for React Web

Copy-paste ready task templates for the Queen to assign to drones.

## Feature Tasks

### Add Product Filtering

```bash
hive-assign drone-1 \
  "Add category filter to products" \
  "Add a category dropdown to Products.tsx that filters products.

   Requirements:
   - Add 'category' field to PRODUCTS data
   - Create CategoryFilter component
   - Filter products when category selected
   - Show 'All' option to clear filter

   Testing:
   1. Start server: npm run dev &
   2. Get URL: hive_get_test_url(port=5173)
   3. Navigate to /products
   4. browser_snapshot() to see the page
   5. Test the filter functionality
   6. browser_screenshot()

   Done when: Category filter works, screenshot saved." \
  "WEB-001"
```

### Add Shopping Cart

```bash
hive-assign drone-1 \
  "Implement basic shopping cart" \
  "Add cart functionality to the product pages.

   Requirements:
   - Add 'Add to Cart' button on each Card
   - Show cart count in Header
   - Store cart state in context/state
   - Cart persists during navigation

   Testing:
   1. Navigate to /products
   2. Click 'Add to Cart' on two products
   3. Verify header shows cart count
   4. Navigate to /contact and back
   5. Verify cart count persists
   6. Screenshot the result" \
  "WEB-002"
```

### Add Form Validation

```bash
hive-assign drone-2 \
  "Add validation to contact form" \
  "Add client-side validation to Contact.tsx form.

   Requirements:
   - Validate email format
   - Minimum message length (10 chars)
   - Show error messages below inputs
   - Disable submit until form is valid

   Testing:
   1. Navigate to /contact
   2. Try submitting empty form
   3. Screenshot error states
   4. Fill valid data
   5. Screenshot success" \
  "WEB-003"
```

## Bug Fix Tasks

### Fix Mobile Navigation

```bash
hive-assign drone-1 \
  "Fix header navigation on mobile" \
  "Header nav links overflow on small screens.

   Fix:
   - Add hamburger menu for mobile
   - Hide nav links below 768px
   - Show mobile menu on hamburger click

   Testing:
   1. browser_resize(width=375, height=667)
   2. browser_snapshot()
   3. Verify hamburger appears
   4. Click hamburger, verify menu shows
   5. Screenshot mobile menu open" \
  "WEB-BUG-001"
```

### Fix Search Clear Button

```bash
hive-assign drone-2 \
  "Add clear button to product search" \
  "Products.tsx search has no way to clear.

   Fix:
   - Add X button inside search input
   - Show only when search has text
   - Clear search and reset results on click

   Testing:
   1. Navigate to /products
   2. Type 'Widget' in search
   3. Click X button
   4. Verify search is cleared
   5. Screenshot showing clear works" \
  "WEB-BUG-002"
```

## Styling Tasks

### Add Dark Mode

```bash
hive-assign drone-1 \
  "Implement dark mode toggle" \
  "Add dark mode support with toggle in header.

   Requirements:
   - Add moon/sun icon button in Header
   - Toggle dark mode CSS variables
   - Persist preference in localStorage
   - Respect system preference initially

   Testing:
   1. Click dark mode toggle
   2. Screenshot in dark mode
   3. Navigate between pages (should persist)
   4. Screenshot another page in dark mode" \
  "WEB-STYLE-001"
```

### Add Loading States

```bash
hive-assign drone-2 \
  "Add skeleton loading to products" \
  "Show skeleton loading while products 'load'.

   Requirements:
   - Create SkeletonCard component
   - Show 4 skeleton cards for 1 second
   - Then fade in real products
   - Subtle pulse animation

   Testing:
   1. Hard refresh the products page
   2. Screenshot skeleton state (be quick!)
   3. Wait for products to load
   4. Screenshot loaded state" \
  "WEB-STYLE-002"
```

## New Page Tasks

### Add About Page

```bash
hive-assign drone-1 \
  "Create About page" \
  "Add new about page with company info.

   Requirements:
   - Create src/pages/About.tsx
   - Add route in App.tsx
   - Add nav link in Header.tsx
   - Include: mission, team section, contact CTA

   Testing:
   1. Navigate to /about
   2. Verify all sections render
   3. Click contact CTA, verify navigation
   4. Screenshot the about page" \
  "WEB-PAGE-001"
```

### Add 404 Page

```bash
hive-assign drone-2 \
  "Create 404 Not Found page" \
  "Add custom 404 page for unknown routes.

   Requirements:
   - Create src/pages/NotFound.tsx
   - Add catch-all route in App.tsx
   - Friendly message with link to home
   - Nice illustration or icon

   Testing:
   1. Navigate to /nonexistent-page
   2. Verify 404 page shows
   3. Click 'Go Home' link
   4. Verify navigation works
   5. Screenshot 404 page" \
  "WEB-PAGE-002"
```

## Testing Workflow Reference

For all tasks, drones should follow this workflow:

```javascript
// 1. Start dev server
// Run: npm run dev &
// Run: hive-log "SERVER RUNNING: http://localhost:5173"

// 2. Get test URL
hive_get_test_url({ port: 5173 })
// Returns: { url: "http://host.docker.internal:15173" }

// 3. Navigate
browser_navigate({ url: "http://host.docker.internal:15173" })

// 4. Understand the page
browser_snapshot()
// Returns accessibility tree with element names and refs

// 5. Interact
browser_click({ element: "Products link", ref: "a[href='/products']" })
browser_wait_for({ text: "Products" })
browser_type({ element: "Search input", ref: "input.search-input", text: "Widget" })

// 6. Capture result
browser_screenshot()

// 7. Complete task
hive_complete_task({
  result: "Feature implemented. Tested navigation and search. See screenshot."
})
```

## Playwright Tips

### Finding Elements

1. **Always use browser_snapshot() first** - It shows you all interactive elements and their refs
2. **Use text content** for buttons and links: `element: "Submit"`
3. **Use CSS selectors** for inputs: `ref: "input#email"`
4. **Use data-testid** for precise targeting: `ref: "[data-testid='submit-btn']"`

### Waiting for Content

```javascript
// Wait for text to appear
browser_wait_for({ text: "Success!" })

// Wait for element to be visible
browser_wait_for({ selector: ".modal" })
```

### Form Filling

```javascript
// Type in input
browser_type({ element: "Email input", ref: "input#email", text: "test@example.com" })

// Select from dropdown
browser_select_option({ element: "Country", ref: "select#country", values: ["US"] })

// Submit form
browser_click({ element: "Submit button", ref: "button[type='submit']" })
```
