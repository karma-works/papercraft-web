# Skill: Automated Model Upload

To enable automated integration testing and better agent collaboration, a global skill `window.loadModel` has been implemented in the Papercraft Web application.

## Usage for AI Agents

Agents can trigger a model upload by executing JavaScript within the browser context. This approach is more robust than interacting with hidden file inputs or native file choosers.

### JavaScript Implementation

```javascript
/**
 * Loads a 3D model into the application.
 * @param {File} file - The file object to upload (OBJ, STL, PDO, PBO).
 */
await window.loadModel(file);
```

### Example: Uploading from Puppeteer/Playwright

If you have a file on the local filesystem, you can transmit it to the browser as a Base64 string and then convert it to a `File` object before calling the skill.

```javascript
const filename = 'model.pdo';
const content = fs.readFileSync(filePath);
const base64 = content.toString('base64');

// Wait for the skill to be initialized (runs in useEffect)
await page.waitForFunction(() => typeof window.loadModel === 'function');

await page.evaluate(async (b64, name) => {
    const response = await fetch(`data:application/octet-stream;base64,${b64}`);
    const blob = await response.blob();
    const file = new File([blob], name);
    await window.loadModel(file);
}, base64, filename);
```

## Benefits
- **No Native Dialogs**: Avoids the "Open File" dialog which often hangs headles browsers.
- **Robust Selectors**: Does not depend on volatile CSS classes or internal React component structures.
- **Direct State Interaction**: Triggers the same logic used by the UI's "Load Model" buttons.

## Troubleshooting
If `window.loadModel` is not found:
1. Ensure the application has finished loading (`networkidle0`).
2. Verify that the `App` component has mounted (the skill is registered in a `useEffect` hook).
3. Check the browser console for any React rendering errors that might have prevented component mounting.
