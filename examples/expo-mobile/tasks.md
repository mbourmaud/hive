# Task Templates for Expo Mobile

Copy-paste ready task templates for the Queen to assign to drones.

## Feature Tasks

### Add Search Functionality

```bash
hive-assign drone-1 \
  "Add search to item list" \
  "Add a TextInput at the top of list.tsx that filters items by title in real-time.

   Requirements:
   - Search input with placeholder 'Search items...'
   - Filter items as user types
   - Show 'No results' if nothing matches

   Testing:
   1. Start Metro: npx expo start --port 8081 &
   2. Get URL: hive_get_test_url(port=8081, protocol='exp')
   3. Boot simulator and open app
   4. Screenshot the search working

   Done when: Search filters items correctly, screenshot saved." \
  "MOBILE-001"
```

### Add Pull-to-Refresh

```bash
hive-assign drone-1 \
  "Add pull-to-refresh to list" \
  "Add RefreshControl to the FlatList in list.tsx.

   Requirements:
   - Show loading indicator when pulling
   - Simulate 1s delay then 'refresh' data
   - Use blue color (#007AFF) for spinner

   Testing:
   1. Start app in simulator
   2. Pull down on list
   3. Screenshot the refresh indicator" \
  "MOBILE-002"
```

### Add Dark Mode

```bash
hive-assign drone-2 \
  "Implement dark mode support" \
  "Add dark mode to all screens using useColorScheme().

   Requirements:
   - Detect system theme
   - Dark backgrounds for dark mode
   - Light text colors in dark mode

   Testing:
   1. Open simulator Settings > Developer > Dark Appearance
   2. Screenshot app in dark mode
   3. Compare with light mode screenshot" \
  "MOBILE-003"
```

## Bug Fix Tasks

### Fix Button Padding

```bash
hive-assign drone-1 \
  "Fix button padding on small screens" \
  "Buttons on profile.tsx are too close to screen edges on iPhone SE.

   Fix:
   - Add horizontal margin to actions container
   - Ensure 16px minimum padding from edges

   Testing:
   1. Boot iPhone SE (3rd generation) simulator
   2. Navigate to Profile tab
   3. Screenshot showing fixed spacing" \
  "MOBILE-BUG-001"
```

### Fix Card Shadow on Android

```bash
hive-assign drone-2 \
  "Fix card shadows not showing on Android" \
  "Card.tsx shadows work on iOS but not Android.

   Fix:
   - Add elevation property for Android
   - Keep iOS shadow properties

   Testing:
   - For now, verify iOS still works
   - Screenshot on iOS simulator" \
  "MOBILE-BUG-002"
```

## Styling Tasks

### Update Color Scheme

```bash
hive-assign drone-1 \
  "Update app color scheme to green" \
  "Change primary color from blue (#007AFF) to green (#34C759).

   Files to update:
   - components/Button.tsx
   - app/(tabs)/profile.tsx (stats)
   - app/_layout.tsx (tab bar)

   Testing:
   1. Start app, navigate all tabs
   2. Screenshot each tab showing new color" \
  "MOBILE-STYLE-001"
```

### Add Loading States

```bash
hive-assign drone-2 \
  "Add loading skeleton to list" \
  "Show skeleton loading animation in list.tsx while data loads.

   Requirements:
   - Create SkeletonCard component
   - Show 3 skeleton cards for 1 second
   - Then show real data
   - Use subtle gray animation

   Testing:
   1. Hard refresh the app
   2. Screenshot the skeleton state
   3. Screenshot after data loads" \
  "MOBILE-STYLE-002"
```

## New Screen Tasks

### Add Settings Screen

```bash
hive-assign drone-1 \
  "Create settings screen" \
  "Add new settings tab with toggle options.

   Requirements:
   - New file: app/(tabs)/settings.tsx
   - Add to tab bar in _layout.tsx
   - Include toggles: Notifications, Dark Mode, Sounds
   - Use Switch component from react-native

   Testing:
   1. Verify tab appears in navigation
   2. Toggle each setting
   3. Screenshot the settings screen" \
  "MOBILE-SCREEN-001"
```

### Add Item Detail Screen

```bash
hive-assign drone-2 \
  "Create item detail screen" \
  "Add dynamic route for item details.

   Requirements:
   - New file: app/item/[id].tsx
   - Show item title, description, and full content
   - Add back button navigation
   - Navigate from Card press in list.tsx

   Testing:
   1. Tap an item in the list
   2. Verify navigation to detail screen
   3. Screenshot the detail view
   4. Go back and screenshot list" \
  "MOBILE-SCREEN-002"
```

## Testing Workflow Reference

For all tasks, drones should follow this workflow:

```javascript
// 1. Start Metro bundler
// Run: npx expo start --port 8081 &
// Run: hive-log "SERVER RUNNING: http://localhost:8081"

// 2. Get test URL
hive_get_test_url({ port: 8081, protocol: "exp" })
// Returns: { url: "exp://localhost:18081" }

// 3. Prepare simulator
ios_list_devices()
ios_boot_device({ device: "iPhone 15" })
ios_install_expo_go({ device: "iPhone 15" })

// 4. Open app
ios_open_url({
  device: "booted",
  url: "exp://localhost:18081"
})

// 5. Wait for app to load
// Wait ~10 seconds for JS bundle

// 6. Take screenshot
ios_screenshot({
  device: "booted",
  outputPath: "/hive-shared/task-result.png"
})

// 7. Complete task
hive_complete_task({
  result: "Feature implemented. Screenshot at /hive-shared/task-result.png"
})
```
