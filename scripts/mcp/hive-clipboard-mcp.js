#!/usr/bin/env node
/**
 * HIVE Clipboard MCP Server
 *
 * Provides clipboard access on macOS (text and images)
 * Uses @modelcontextprotocol/sdk with SSE transport
 */

import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { SSEServerTransport } from '@modelcontextprotocol/sdk/server/sse.js';
import express from 'express';
import { execSync, spawn } from 'child_process';
import fs from 'fs';
import path from 'path';
import os from 'os';

// Parse command line arguments
const args = process.argv.slice(2);
let port = 8933;
for (let i = 0; i < args.length; i++) {
  if (args[i] === '--port' && args[i + 1]) {
    port = parseInt(args[i + 1], 10);
  }
}

// Helper: Execute command
function execCommand(cmd) {
  try {
    return execSync(cmd, { encoding: 'utf8', maxBuffer: 50 * 1024 * 1024 }).trim();
  } catch (error) {
    throw new Error(error.message);
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
      try
        set pngData to the clipboard as «class PNGf»
        return true
      on error
        return false
      end try
    `;
    const result = execCommand(`osascript -e '${script}'`);
    return result === 'true';
  } catch {
    return false;
  }
}

// Create MCP Server
const server = new McpServer({
  name: 'hive-clipboard-mcp',
  version: '1.0.0'
});

// Tool: clipboard_read_text
server.tool(
  'clipboard_read_text',
  {
    description: 'Read text content from the macOS clipboard',
    inputSchema: {
      type: 'object',
      properties: {}
    }
  },
  async () => {
    try {
      const text = execCommand('pbpaste');
      return {
        content: [{
          type: 'text',
          text: text || '(clipboard is empty or contains non-text data)'
        }]
      };
    } catch (error) {
      return {
        content: [{ type: 'text', text: `Error reading clipboard: ${error.message}` }],
        isError: true
      };
    }
  }
);

// Tool: clipboard_write_text
server.tool(
  'clipboard_write_text',
  {
    description: 'Write text to the macOS clipboard',
    inputSchema: {
      type: 'object',
      properties: {
        text: { type: 'string', description: 'Text to write to clipboard' }
      },
      required: ['text']
    }
  },
  async ({ text }) => {
    if (!text) {
      return {
        content: [{ type: 'text', text: 'Error: text parameter is required' }],
        isError: true
      };
    }

    try {
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
);

// Tool: clipboard_read_image
server.tool(
  'clipboard_read_image',
  {
    description: 'Read image from clipboard (requires pngpaste: brew install pngpaste)',
    inputSchema: {
      type: 'object',
      properties: {}
    }
  },
  async () => {
    try {
      if (!isPngpasteInstalled()) {
        return {
          content: [{
            type: 'text',
            text: 'Error: pngpaste is not installed. Install it with: brew install pngpaste'
          }],
          isError: true
        };
      }

      if (!clipboardHasImage()) {
        return {
          content: [{
            type: 'text',
            text: 'Clipboard does not contain an image. Use clipboard_read_text for text content.'
          }],
          isError: true
        };
      }

      const imageData = execSync('pngpaste - | base64', {
        encoding: 'utf8',
        maxBuffer: 50 * 1024 * 1024
      }).trim();

      if (!imageData) {
        return {
          content: [{ type: 'text', text: 'Clipboard does not contain an image' }],
          isError: true
        };
      }

      return {
        content: [{
          type: 'image',
          data: imageData,
          mimeType: 'image/png'
        }]
      };
    } catch (error) {
      return {
        content: [{ type: 'text', text: `Error reading image from clipboard: ${error.message}` }],
        isError: true
      };
    }
  }
);

// Tool: clipboard_write_image
server.tool(
  'clipboard_write_image',
  {
    description: 'Write image to clipboard from base64 data or file path',
    inputSchema: {
      type: 'object',
      properties: {
        base64: { type: 'string', description: 'Base64-encoded PNG image data' },
        filePath: { type: 'string', description: 'Path to image file' }
      }
    }
  },
  async ({ base64, filePath }) => {
    if (!base64 && !filePath) {
      return {
        content: [{ type: 'text', text: 'Error: either base64 or filePath parameter is required' }],
        isError: true
      };
    }

    try {
      let imagePath = filePath;
      let tempFile = null;

      if (base64) {
        tempFile = path.join(os.tmpdir(), `hive-clipboard-${Date.now()}.png`);
        const imageBuffer = Buffer.from(base64, 'base64');
        fs.writeFileSync(tempFile, imageBuffer);
        imagePath = tempFile;
      }

      if (!fs.existsSync(imagePath)) {
        return {
          content: [{ type: 'text', text: `Error: file not found: ${imagePath}` }],
          isError: true
        };
      }

      const script = `
        set theFile to POSIX file "${imagePath}"
        set theImage to read theFile as «class PNGf»
        set the clipboard to theImage
      `;
      execCommand(`osascript -e '${script}'`);

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
);

// Tool: clipboard_get_formats
server.tool(
  'clipboard_get_formats',
  {
    description: 'Get available formats in the clipboard (text, image types, etc.)',
    inputSchema: {
      type: 'object',
      properties: {}
    }
  },
  async () => {
    try {
      const script = `
        try
          set clipInfo to {}
          try
            set theText to the clipboard as text
            set end of clipInfo to "text (length: " & (length of theText) & " chars)"
          end try
          try
            set pngData to the clipboard as «class PNGf»
            set end of clipInfo to "PNG image"
          end try
          try
            set jpegData to the clipboard as «class JPEG»
            set end of clipInfo to "JPEG image"
          end try
          try
            set tiffData to the clipboard as «class TIFF»
            set end of clipInfo to "TIFF image"
          end try
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
);

// Tool: clipboard_clear
server.tool(
  'clipboard_clear',
  {
    description: 'Clear the clipboard contents',
    inputSchema: {
      type: 'object',
      properties: {}
    }
  },
  async () => {
    try {
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
);

// Express server with SSE transport
const app = express();
app.use(express.json());

// Track active transports
const transports = new Map();

app.get('/sse', async (req, res) => {
  const transport = new SSEServerTransport('/messages', res);
  // Use the sessionId generated by SSEServerTransport (UUID sent to client)
  const sessionId = transport.sessionId;
  transports.set(sessionId, transport);

  res.on('close', () => {
    transports.delete(sessionId);
  });

  await server.connect(transport);
});

app.post('/messages', async (req, res) => {
  const sessionId = req.query.sessionId;
  const transport = sessionId ? transports.get(sessionId) : [...transports.values()].pop();

  if (transport) {
    await transport.handlePostMessage(req, res, req.body);
  } else {
    res.status(400).json({ error: 'No active SSE connection' });
  }
});

app.get('/health', (req, res) => {
  res.json({ status: 'ok', connections: transports.size, pngpaste: isPngpasteInstalled() });
});

app.listen(port, () => {
  console.log(`HIVE Clipboard MCP Server running on http://localhost:${port}`);
  console.log(`SSE endpoint: http://localhost:${port}/sse`);
  console.log(`Messages endpoint: http://localhost:${port}/messages`);
  console.log('');
  console.log('Available tools:');
  console.log('  - clipboard_read_text: Read text from clipboard');
  console.log('  - clipboard_write_text: Write text to clipboard');
  console.log('  - clipboard_read_image: Read image from clipboard (requires pngpaste)');
  console.log('  - clipboard_write_image: Write image to clipboard');
  console.log('  - clipboard_get_formats: Get clipboard formats');
  console.log('  - clipboard_clear: Clear clipboard');
  console.log('');
  console.log(`pngpaste: ${isPngpasteInstalled() ? 'installed' : 'not installed (brew install pngpaste for image support)'}`);
});
