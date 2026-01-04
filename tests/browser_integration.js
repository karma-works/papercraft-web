import puppeteer from 'puppeteer';
import path from 'path';
import fs from 'fs';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

async function runTest() {
    console.log('🚀 Starting Browser Integration Test...');

    const browser = await puppeteer.launch({
        headless: "new",
        args: ['--no-sandbox', '--disable-setuid-sandbox']
    });

    const page = await browser.newPage();
    const url = 'http://localhost:5173';

    try {
        console.log(`🔗 Navigating to ${url}...`);
        await page.goto(url, { waitUntil: 'networkidle0' });

        // 1. Import PDO file
        console.log('📂 Importing sphere.pdo...');
        const pdoPath = path.resolve(__dirname, '../backend/examples/sphere.pdo');

        // Find the file input
        // The FileTrigger in react-aria-components usually creates a hidden input
        const [fileChooser] = await Promise.all([
            page.waitForFileChooser(),
            page.evaluate(() => {
                const buttons = Array.from(document.querySelectorAll('button'));
                const target = buttons.find(b => b.textContent.includes('Browse Files') || b.textContent.includes('Load Model'));
                if (target) target.click();
            })
        ]);
        await fileChooser.accept([pdoPath]);

        // Wait for upload and model loading
        console.log('⏳ Waiting for model to load...');
        await page.waitForNetworkIdle();

        // 2. Export to SVG
        console.log('📤 Exporting to SVG...');

        // Set up a listener for the download or API call via window.open
        await page.evaluate(() => {
            window.exportTriggered = false;
            window.originalOpen = window.open;
            window.open = (url) => {
                if (url && url.includes('/api/export?format=svg')) {
                    window.exportTriggered = true;
                }
                return { close: () => { } };
            };
        });

        // Click the SVG button
        await page.evaluate(() => {
            const buttons = Array.from(document.querySelectorAll('button'));
            const svgButton = buttons.find(b => b.title === 'Export SVG' || (b.textContent && b.textContent.includes('SVG')));
            if (svgButton) svgButton.click();
        });

        // Give it a moment to trigger
        await new Promise(resolve => setTimeout(resolve, 2000));

        // Check if export was triggered
        const exportTriggered = await page.evaluate(() => window.exportTriggered);

        // 3. Assert the SVG was exported
        if (exportTriggered) {
            console.log('✨ Integration Test PASSED: PDO imported and SVG export triggered.');
        } else {
            throw new Error('Integration Test FAILED: SVG export was not triggered.');
        }

    } catch (error) {
        console.error('❌ Test failed:', error.message);
        process.exit(1);
    } finally {
        await browser.close();
    }
}

runTest();
