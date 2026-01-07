const { app, BrowserWindow, shell } = require('electron')
const path = require('path')
const { spawn } = require('child_process')
const http = require('http')
const fs = require('fs')
const url = require('url')

let mainWindow = null
let hubProcess = null
let webServer = null

const isDev = !app.isPackaged
const HUB_PORT = 7433
const WEB_PORT = 7434
const HUB_URL = `http://localhost:${HUB_PORT}`

function getHiveBinaryPath() {
  if (isDev) {
    return path.join(__dirname, '../../hive')
  }
  return path.join(process.resourcesPath, 'hive')
}

function getDistPath() {
  if (isDev) {
    return path.join(__dirname, '../dist')
  }
  return path.join(process.resourcesPath, 'app.asar', 'dist')
}

function checkHubRunning() {
  return new Promise((resolve) => {
    const req = http.get(`${HUB_URL}/agents`, (res) => {
      res.resume()
      resolve(true)
    })
    req.on('error', () => resolve(false))
    req.setTimeout(500, () => {
      req.destroy()
      resolve(false)
    })
  })
}

async function startHub() {
  const isRunning = await checkHubRunning()
  if (isRunning) {
    console.log('Hub already running')
    return true
  }

  console.log('Starting Hub server...')
  const hivePath = getHiveBinaryPath()
  console.log('Hive binary path:', hivePath)
  
  try {
    hubProcess = spawn(hivePath, ['hub'], {
      stdio: ['ignore', 'pipe', 'pipe'],
      detached: false,
    })

    hubProcess.stdout.on('data', (data) => {
      console.log('Hub stdout:', data.toString())
    })

    hubProcess.stderr.on('data', (data) => {
      console.error('Hub stderr:', data.toString())
    })

    hubProcess.on('error', (err) => {
      console.error('Failed to start hub:', err.message)
    })

    hubProcess.on('exit', (code) => {
      console.log('Hub process exited with code:', code)
      hubProcess = null
    })

    for (let i = 0; i < 50; i++) {
      await new Promise(r => setTimeout(r, 100))
      const running = await checkHubRunning()
      if (running) {
        console.log('Hub started successfully')
        return true
      }
    }

    console.error('Hub failed to start within 5 seconds')
    return false
  } catch (err) {
    console.error('Error starting hub:', err)
    return false
  }
}

function stopHub() {
  if (hubProcess) {
    console.log('Stopping Hub server...')
    hubProcess.kill('SIGTERM')
    hubProcess = null
  }
}

function getMimeType(filePath) {
  const ext = path.extname(filePath).toLowerCase()
  const mimeTypes = {
    '.html': 'text/html',
    '.js': 'application/javascript',
    '.css': 'text/css',
    '.json': 'application/json',
    '.png': 'image/png',
    '.jpg': 'image/jpeg',
    '.svg': 'image/svg+xml',
    '.ico': 'image/x-icon',
    '.woff': 'font/woff',
    '.woff2': 'font/woff2',
  }
  return mimeTypes[ext] || 'application/octet-stream'
}

function proxyRequest(req, res, targetPath) {
  const options = {
    hostname: 'localhost',
    port: HUB_PORT,
    path: targetPath,
    method: req.method,
    headers: { ...req.headers, host: `localhost:${HUB_PORT}` },
  }

  const proxyReq = http.request(options, (proxyRes) => {
    res.writeHead(proxyRes.statusCode, proxyRes.headers)
    proxyRes.pipe(res)
  })

  proxyReq.on('error', (err) => {
    res.writeHead(502)
    res.end('Hub not available')
  })

  req.pipe(proxyReq)
}

function startWebServer() {
  const distPath = getDistPath()
  console.log('Dist path:', distPath)
  
  webServer = http.createServer((req, res) => {
    const parsedUrl = url.parse(req.url, true)
    const pathname = parsedUrl.pathname

    res.setHeader('Access-Control-Allow-Origin', '*')
    res.setHeader('Access-Control-Allow-Methods', 'GET, POST, DELETE, OPTIONS')
    res.setHeader('Access-Control-Allow-Headers', 'Content-Type')

    if (req.method === 'OPTIONS') {
      res.writeHead(200)
      res.end()
      return
    }

    if (pathname.startsWith('/api/') || pathname === '/ws') {
      const hubPath = pathname.replace('/api', '')
      proxyRequest(req, res, hubPath)
      return
    }

    let filePath = path.join(distPath, pathname === '/' ? 'index.html' : pathname)
    
    fs.readFile(filePath, (err, data) => {
      if (err) {
        fs.readFile(path.join(distPath, 'index.html'), (err2, data2) => {
          if (err2) {
            res.writeHead(404)
            res.end('Not found')
            return
          }
          res.writeHead(200, { 'Content-Type': 'text/html' })
          res.end(data2)
        })
        return
      }
      res.writeHead(200, { 'Content-Type': getMimeType(filePath) })
      res.end(data)
    })
  })

  webServer.listen(WEB_PORT, () => {
    console.log(`Web server running at http://localhost:${WEB_PORT}`)
  })
}

function stopWebServer() {
  if (webServer) {
    webServer.close()
    webServer = null
  }
}

async function createWindow() {
  const hubStarted = await startHub()
  console.log('Hub started:', hubStarted)
  
  if (!isDev) {
    startWebServer()
  }

  mainWindow = new BrowserWindow({
    width: 1400,
    height: 900,
    minWidth: 900,
    minHeight: 600,
    title: 'Hive Monitor',
    backgroundColor: '#0D0D0F',
    titleBarStyle: 'default',
    webPreferences: {
      preload: path.join(__dirname, 'preload.cjs'),
      contextIsolation: true,
      nodeIntegration: false,
    },
  })

  mainWindow.webContents.setWindowOpenHandler(({ url }) => {
    shell.openExternal(url)
    return { action: 'deny' }
  })

  if (isDev) {
    mainWindow.loadURL('http://localhost:7435')
    mainWindow.webContents.openDevTools()
  } else {
    mainWindow.loadURL(`http://localhost:${WEB_PORT}`)
  }

  mainWindow.on('closed', () => {
    mainWindow = null
  })
}

app.whenReady().then(createWindow)

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    stopWebServer()
    stopHub()
    app.quit()
  }
})

app.on('before-quit', () => {
  stopWebServer()
  stopHub()
})

app.on('activate', () => {
  if (mainWindow === null) {
    createWindow()
  }
})
