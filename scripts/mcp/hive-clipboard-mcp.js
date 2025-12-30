#!/usr/bin/env node
/**
 * Hive Clipboard MCP Server
 *
 * Provides clipboard access (text and images) to Claude Code agents
 * running in Docker containers via SSE transport.
 *
 * Usage: node hive-clipboard-mcp.js --port 8933
 *
 * Prerequisites:
 * - macOS (uses pbcopy/pbpaste)
 * - For image support: brew install pngpaste
 */

const http = require('http');
const { spawn, execSync } = require('child_process');
const fs = require('fs');
const path = require('path');
const os = require('os');

// Parse command line arguments
const args = process.argv.slice(2);
let port = 8933;

for (let i = 0; i < args.length; i++) {
    if (args[i] === '--port' && args[i + 1]) {
        port = parseInt(args[i + 1], 10);
    }
}

// SSE client management
const sseClients = new Set();

// MCP Protocol implementation
const JSONRPC_VERSION = '2.0';

// Tool definitions
const tools = [
    {
        name: 'clipboard_read_text',
        description: 'Read text content from the macOS clipboard',
        inputSchema: {
            type: 'object',
            properties: {},
            required: []
        }
    },
    {
        name: 'clipboard_write_text',
        description: 'Write text content to the macOS clipboard',
        inputSchema: {
            type: 'object',
            properties: {
                text: {
                    type: 'string',
                    description: 'The text to write to the clipboard'
                }
            },
            required: ['text']
        }
    },
    {
        name: 'clipboard_read_image',
        description: 'Read image from the macOS clipboard. Returns base64-encoded PNG. Requires pngpaste to be installed (brew install pngpaste).',
        inputSchema: {
            type: 'object',
            properties: {},
            required: []
        }
    },
    {
        name: 'clipboard_write_image',
        description: 'Write an image to the macOS clipboard from a base64-encoded string or file path',
        inputSchema: {
            type: 'object',
            properties: {
                base64: {
                    type: 'string',
                    description: 'Base64-encoded image data (PNG or JPEG)'
                },
                filePath: {
                    type: 'string',
                    description: 'Path to an image file to copy to clipboard'
                }
            },
            required: []
        }
    },
    {
        name: 'clipboard_get_formats',
        description: 'Get the available data formats currently in the clipboard',
        inputSchema: {
            type: 'object',
            properties: {},
            required: []
        }
    },
    {
        name: 'clipboard_clear',
        description: 'Clear the clipboard contents',
        inputSchema: {
            type: 'object',
            properties: {},
            required: []
        }
    }
];

// Helper: Execute command and return stdout
function execCommand(cmd, options = {}) {
    try {
        return execSync(cmd, { encoding: 'utf8', maxBuffer: 50 * 1024 * 1024, ...options }).trim();
    } catch (error) {
        throw new Error(error.stderr || error.message);
    }
}

// Helper: Check if pngpaste is installed
function isPngpasteInstalled() {
    try {
        execSync('which pngpaste', { encoding: 'utf8' });
        return true;
    } catch {
        return false;
    }
}

// Helper: Check if clipboard has image
function clipboardHasImage() {
    try {
        const script = `
            tell application "System Events"
                try
                    set theClasses to (class of (the clipboard as record))
                    return theClasses contains «class PNGf» or theClasses contains «class JPEG» or theClasses contains «class TIFF»
                on error
                    return false
                end try
            end tell
        `;
        const result = execCommand(`osascript -e '${script}'`);
        return result === 'true';
    } catch {
        return false;
    }
}

// Tool implementations
async function clipboardReadText() {
    try {
        const text = execCommand('pbpaste');
        return {
            content: [
                {
                    type: 'text',
                    text: text || '(clipboard is empty or contains non-text data)'
                }
            ]
        };
    } catch (error) {
        return {
            content: [{ type: 'text', text: `Error reading clipboard: ${error.message}` }],
            isError: true
        };
    }
}

async function clipboardWriteText(args) {
    const { text } = args;
    if (!text) {
        return {
            content: [{ type: 'text', text: 'Error: text parameter is required' }],
            isError: true
        };
    }

    try {
        // Use process spawn to handle special characters safely
        const proc = spawn('pbcopy', [], { stdio: ['pipe', 'pipe', 'pipe'] });
        proc.stdin.write(text);
        proc.stdin.end();

        await new Promise((resolve, reject) => {
            proc.on('close', (code) => {
                if (code === 0) resolve();
                else reject(new Error(`pbcopy exited with code ${code}`));
            });
            proc.on('error', reject);
        });

        return {
            content: [{ type: 'text', text: `Successfully wrote ${text.length} characters to clipboard` }]
        };
    } catch (error) {
        return {
            content: [{ type: 'text', text: `Error writing to clipboard: ${error.message}` }],
            isError: true
        };
    }
}

async function clipboardReadImage() {
    try {
        // Check if pngpaste is installed
        if (!isPngpasteInstalled()) {
            return {
                content: [{
                    type: 'text',
                    text: 'Error: pngpaste is not installed. Install it with: brew install pngpaste'
                }],
                isError: true
            };
        }

        // Check if clipboard has image
        if (!clipboardHasImage()) {
            return {
                content: [{
                    type: 'text',
                    text: 'Clipboard does not contain an image. Use clipboard_read_text for text content.'
                }],
                isError: true
            };
        }

        // Use pngpaste to read image to stdout and convert to base64
        const imageData = execSync('pngpaste - | base64', {
            encoding: 'utf8',
            maxBuffer: 50 * 1024 * 1024 // 50MB buffer for large images
        }).trim();

        if (!imageData) {
            return {
                content: [{ type: 'text', text: 'Clipboard does not contain an image' }],
                isError: true
            };
        }

        return {
            content: [
                {
                    type: 'image',
                    data: imageData,
                    mimeType: 'image/png'
                }
            ]
        };
    } catch (error) {
        return {
            content: [{ type: 'text', text: `Error reading image from clipboard: ${error.message}` }],
            isError: true
        };
    }
}

async function clipboardWriteImage(args) {
    const { base64, filePath } = args;

    if (!base64 && !filePath) {
        return {
            content: [{ type: 'text', text: 'Error: either base64 or filePath parameter is required' }],
            isError: true
        };
    }

    try {
        let imagePath = filePath;
        let tempFile = null;

        // If base64 provided, write to temp file
        if (base64) {
            tempFile = path.join(os.tmpdir(), `hive-clipboard-${Date.now()}.png`);
            const imageBuffer = Buffer.from(base64, 'base64');
            fs.writeFileSync(tempFile, imageBuffer);
            imagePath = tempFile;
        }

        // Verify file exists
        if (!fs.existsSync(imagePath)) {
            return {
                content: [{ type: 'text', text: `Error: file not found: ${imagePath}` }],
                isError: true
            };
        }

        // Use osascript to set clipboard to image
        const script = `
            set theFile to POSIX file "${imagePath}"
            set theImage to read theFile as «class PNGf»
            set the clipboard to theImage
        `;
        execCommand(`osascript -e '${script}'`);

        // Clean up temp file
        if (tempFile) {
            fs.unlinkSync(tempFile);
        }

        return {
            content: [{ type: 'text', text: 'Successfully wrote image to clipboard' }]
        };
    } catch (error) {
        return {
            content: [{ type: 'text', text: `Error writing image to clipboard: ${error.message}` }],
            isError: true
        };
    }
}

async function clipboardGetFormats() {
    try {
        // Use osascript to get clipboard formats
        const script = `
            try
                set clipInfo to {}

                -- Check for text
                try
                    set theText to the clipboard as text
                    set end of clipInfo to "text (length: " & (length of theText) & " chars)"
                end try

                -- Check for PNG image
                try
                    set pngData to the clipboard as «class PNGf»
                    set end of clipInfo to "PNG image"
                end try

                -- Check for JPEG image
                try
                    set jpegData to the clipboard as «class JPEG»
                    set end of clipInfo to "JPEG image"
                end try

                -- Check for TIFF image
                try
                    set tiffData to the clipboard as «class TIFF»
                    set end of clipInfo to "TIFF image"
                end try

                -- Check for file references
                try
                    set fileList to the clipboard as «class furl»
                    set end of clipInfo to "File reference"
                end try

                if (count of clipInfo) = 0 then
                    return "Clipboard is empty"
                else
                    return clipInfo as text
                end if
            on error errMsg
                return "Error: " & errMsg
            end try
        `;

        const result = execCommand(`osascript -e '${script}'`);

        return {
            content: [{
                type: 'text',
                text: `Clipboard formats:\n${result}`
            }]
        };
    } catch (error) {
        return {
            content: [{ type: 'text', text: `Error getting clipboard formats: ${error.message}` }],
            isError: true
        };
    }
}

async function clipboardClear() {
    try {
        // Use osascript to clear clipboard
        execCommand(`osascript -e 'set the clipboard to ""'`);

        return {
            content: [{ type: 'text', text: 'Clipboard cleared' }]
        };
    } catch (error) {
        return {
            content: [{ type: 'text', text: `Error clearing clipboard: ${error.message}` }],
            isError: true
        };
    }
}

// Execute tool
async function executeTool(name, args) {
    switch (name) {
        case 'clipboard_read_text':
            return await clipboardReadText();
        case 'clipboard_write_text':
            return await clipboardWriteText(args);
        case 'clipboard_read_image':
            return await clipboardReadImage();
        case 'clipboard_write_image':
            return await clipboardWriteImage(args);
        case 'clipboard_get_formats':
            return await clipboardGetFormats();
        case 'clipboard_clear':
            return await clipboardClear();
        default:
            return {
                content: [{ type: 'text', text: `Unknown tool: ${name}` }],
                isError: true
            };
    }
}

// Handle MCP JSON-RPC request
async function handleMCPRequest(request) {
    const { method, params, id } = request;

    switch (method) {
        case 'initialize':
            return {
                jsonrpc: JSONRPC_VERSION,
                id,
                result: {
                    protocolVersion: '2024-11-05',
                    capabilities: {
                        tools: {}
                    },
                    serverInfo: {
                        name: 'hive-clipboard-mcp',
                        version: '1.0.0'
                    }
                }
            };

        case 'notifications/initialized':
            // No response needed for notifications
            return null;

        case 'tools/list':
            return {
                jsonrpc: JSONRPC_VERSION,
                id,
                result: { tools }
            };

        case 'tools/call':
            const { name, arguments: toolArgs } = params;
            const result = await executeTool(name, toolArgs || {});
            return {
                jsonrpc: JSONRPC_VERSION,
                id,
                result
            };

        default:
            return {
                jsonrpc: JSONRPC_VERSION,
                id,
                error: {
                    code: -32601,
                    message: `Method not found: ${method}`
                }
            };
    }
}

// Broadcast to all SSE clients
function broadcastSSE(message) {
    const data = JSON.stringify(message);
    for (const client of sseClients) {
        client.write(`data: ${data}\n\n`);
    }
}

// HTTP server
const server = http.createServer(async (req, res) => {
    // CORS headers
    res.setHeader('Access-Control-Allow-Origin', '*');
    res.setHeader('Access-Control-Allow-Methods', 'GET, POST, OPTIONS');
    res.setHeader('Access-Control-Allow-Headers', 'Content-Type');

    if (req.method === 'OPTIONS') {
        res.writeHead(200);
        res.end();
        return;
    }

    const url = new URL(req.url, `http://localhost:${port}`);

    // Health check
    if (url.pathname === '/health') {
        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({
            status: 'ok',
            server: 'hive-clipboard-mcp',
            pngpaste: isPngpasteInstalled() ? 'installed' : 'not installed'
        }));
        return;
    }

    // SSE endpoint
    if (url.pathname === '/sse') {
        res.writeHead(200, {
            'Content-Type': 'text/event-stream',
            'Cache-Control': 'no-cache',
            'Connection': 'keep-alive'
        });

        // Send initial endpoint message
        const endpointMessage = {
            jsonrpc: JSONRPC_VERSION,
            method: 'endpoint',
            params: {
                endpoint: `http://localhost:${port}/mcp`
            }
        };
        res.write(`data: ${JSON.stringify(endpointMessage)}\n\n`);

        sseClients.add(res);

        req.on('close', () => {
            sseClients.delete(res);
        });

        return;
    }

    // MCP endpoint
    if (url.pathname === '/mcp' && req.method === 'POST') {
        let body = '';
        req.on('data', chunk => { body += chunk; });
        req.on('end', async () => {
            try {
                const request = JSON.parse(body);
                const response = await handleMCPRequest(request);

                if (response) {
                    res.writeHead(200, { 'Content-Type': 'application/json' });
                    res.end(JSON.stringify(response));

                    // Broadcast to SSE clients
                    broadcastSSE(response);
                } else {
                    res.writeHead(204);
                    res.end();
                }
            } catch (error) {
                res.writeHead(400, { 'Content-Type': 'application/json' });
                res.end(JSON.stringify({
                    jsonrpc: JSONRPC_VERSION,
                    error: {
                        code: -32700,
                        message: 'Parse error',
                        data: error.message
                    }
                }));
            }
        });
        return;
    }

    // Not found
    res.writeHead(404, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ error: 'Not found' }));
});

// Start server
server.listen(port, () => {
    console.log(`Hive Clipboard MCP server running on port ${port}`);
    console.log(`SSE endpoint: http://localhost:${port}/sse`);
    console.log(`MCP endpoint: http://localhost:${port}/mcp`);
    console.log(`Health check: http://localhost:${port}/health`);
    console.log(`pngpaste: ${isPngpasteInstalled() ? 'installed' : 'not installed (image support disabled)'}`);
});

// Graceful shutdown
process.on('SIGTERM', () => {
    console.log('Received SIGTERM, shutting down...');
    server.close(() => {
        process.exit(0);
    });
});

process.on('SIGINT', () => {
    console.log('Received SIGINT, shutting down...');
    server.close(() => {
        process.exit(0);
    });
});
