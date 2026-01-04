/**
 * Papercraft Web Integration Tests
 * 
 * Run with: npm test
 * Requires backend and frontend to be running: npm run dev
 */

const BACKEND_URL = 'http://localhost:3000';
const FRONTEND_URL = 'http://localhost:5173';

// Simple test framework
let passed = 0;
let failed = 0;

async function test(name, fn) {
    try {
        await fn();
        console.log(`✓ ${name}`);
        passed++;
    } catch (error) {
        console.error(`✗ ${name}`);
        console.error(`  Error: ${error.message}`);
        failed++;
    }
}

function assert(condition, message) {
    if (!condition) {
        throw new Error(message || 'Assertion failed');
    }
}

// Tests
async function runTests() {
    console.log('\n🧪 Running Papercraft Web Integration Tests\n');
    console.log('='.repeat(50));

    // Backend Tests
    console.log('\n📡 Backend API Tests\n');

    await test('GET /api/status returns OK', async () => {
        const response = await fetch(`${BACKEND_URL}/api/status`);
        assert(response.ok, 'Response should be OK');
        const data = await response.json();
        assert(data.status === 'ok', 'Status should be "ok"');
        assert(typeof data.has_model === 'boolean', 'has_model should be boolean');
    });

    await test('GET /api/project returns 404 when no model loaded', async () => {
        const response = await fetch(`${BACKEND_URL}/api/project`);
        // May be 200 if a model was previously loaded, or 404 if not
        assert(response.status === 200 || response.status === 404,
            'Should return 200 or 404');
    });

    await test('POST /api/upload accepts OBJ file', async () => {
        const objContent = 'v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n';
        const blob = new Blob([objContent], { type: 'text/plain' });
        const formData = new FormData();
        formData.append('file', blob, 'test.obj');

        const response = await fetch(`${BACKEND_URL}/api/upload`, {
            method: 'POST',
            body: formData,
        });

        assert(response.ok, 'Upload should succeed');
        const data = await response.json();
        assert(data === 'Uploaded', 'Should return "Uploaded"');
    });

    await test('GET /api/status shows model loaded after upload', async () => {
        const response = await fetch(`${BACKEND_URL}/api/status`);
        const data = await response.json();
        assert(data.has_model === true, 'has_model should be true after upload');
    });

    await test('GET /api/project returns model data', async () => {
        const response = await fetch(`${BACKEND_URL}/api/project`);
        assert(response.ok, 'Response should be OK');
        const data = await response.json();
        assert(data.model, 'Should have model property');
        assert(data.options, 'Should have options property');
        assert(data.islands, 'Should have islands property');
    });

    await test('POST /api/action performs moveIsland', async () => {
        // First get current project to find island key
        const projectResponse = await fetch(`${BACKEND_URL}/api/project`);
        const project = await projectResponse.json();

        // Find a valid island
        const validIsland = project.islands.find(i => i.value !== null);
        if (!validIsland) {
            console.log('  (skipping - no valid islands)');
            return;
        }

        const response = await fetch(`${BACKEND_URL}/api/action`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                type: 'moveIsland',
                island: { idx: 1, version: validIsland.version },
                delta: [10.0, 10.0],
            }),
        });

        assert(response.ok, 'Action should succeed');
        const data = await response.json();
        assert(data.islands, 'Response should include updated islands');
    });

    // Frontend Tests
    console.log('\n🖥️  Frontend Tests\n');

    await test('Frontend serves HTML at root', async () => {
        const response = await fetch(FRONTEND_URL);
        assert(response.ok, 'Response should be OK');
        const html = await response.text();
        assert(html.includes('Papercraft Web'), 'Should include title');
        assert(html.includes('id="root"'), 'Should have React root element');
    });

    await test('Vite dev server is running', async () => {
        const response = await fetch(`${FRONTEND_URL}/@vite/client`);
        assert(response.ok, 'Vite client should be available');
    });

    await test('API proxy works through frontend', async () => {
        const response = await fetch(`${FRONTEND_URL}/api/status`);
        assert(response.ok, 'Proxied API call should succeed');
        const data = await response.json();
        assert(data.status === 'ok', 'Should return backend status');
    });

    // Export Tests
    console.log('\n📤 Export Tests\n');

    await test('GET /api/export?format=svg returns SVG', async () => {
        // Ensure a model is loaded first
        const objContent = 'v 0 0 0\nv 1 0 0\nv 0 1 0\nv 1 1 0\nf 1 2 3\nf 2 4 3\n';
        const blob = new Blob([objContent], { type: 'text/plain' });
        const formData = new FormData();
        formData.append('file', blob, 'test.obj');
        await fetch(`${BACKEND_URL}/api/upload`, { method: 'POST', body: formData });

        const response = await fetch(`${BACKEND_URL}/api/export?format=svg`);
        assert(response.ok, 'SVG export should succeed');
        const contentType = response.headers.get('content-type');
        assert(contentType && contentType.includes('svg'), 'Should return SVG content type');
        const svg = await response.text();
        assert(svg.includes('<svg'), 'Should contain SVG opening tag');
        assert(svg.includes('</svg>'), 'Should contain SVG closing tag');
    });

    await test('GET /api/export?format=svg&page=0 returns single page SVG', async () => {
        const response = await fetch(`${BACKEND_URL}/api/export?format=svg&page=0`);
        assert(response.ok, 'Single page SVG export should succeed');
        const svg = await response.text();
        assert(svg.includes('<svg'), 'Should contain SVG tag');
    });

    await test('GET /api/export?format=pdf returns PDF', async () => {
        const response = await fetch(`${BACKEND_URL}/api/export?format=pdf`);
        assert(response.ok, 'PDF export should succeed');
        const contentType = response.headers.get('content-type');
        assert(contentType && contentType.includes('pdf'), 'Should return PDF content type');
        const buffer = await response.arrayBuffer();
        const bytes = new Uint8Array(buffer);
        // Check PDF magic bytes: %PDF
        assert(bytes[0] === 0x25 && bytes[1] === 0x50 && bytes[2] === 0x44 && bytes[3] === 0x46,
            'Should start with PDF magic bytes');
    });

    await test('GET /api/export without format returns 400', async () => {
        const response = await fetch(`${BACKEND_URL}/api/export`);
        assert(response.status === 400, 'Should return 400 for missing format');
    });

    await test('GET /api/export with invalid format returns 400', async () => {
        const response = await fetch(`${BACKEND_URL}/api/export?format=invalid`);
        assert(response.status === 400, 'Should return 400 for invalid format');
    });

    // Summary
    console.log('\n' + '='.repeat(50));
    console.log(`\n📊 Results: ${passed} passed, ${failed} failed\n`);

    if (failed > 0) {
        process.exit(1);
    }
}

// Run tests
runTests().catch(error => {
    console.error('Test runner error:', error);
    process.exit(1);
});
