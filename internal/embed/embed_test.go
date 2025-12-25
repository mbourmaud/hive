package embed

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// TestGetFile tests reading individual files from embedded filesystem
func TestGetFile(t *testing.T) {
	tests := []struct {
		name     string
		path     string
		wantErr  bool
		contains string // substring that should be in the file content
	}{
		{
			name:     "read .env.example",
			path:     ".env.example",
			wantErr:  false,
			contains: "GIT_USER_EMAIL",
		},
		{
			name:     "read docker-compose.yml",
			path:     "docker-compose.yml",
			wantErr:  false,
			contains: "services:",
		},
		{
			name:     "read backends.py",
			path:     "backends.py",
			wantErr:  false,
			contains: "ClaudeBackend",
		},
		{
			name:    "non-existent file",
			path:    "nonexistent-file.txt",
			wantErr: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			content, err := GetFile(tt.path)

			if (err != nil) != tt.wantErr {
				t.Errorf("GetFile(%q) error = %v, wantErr %v", tt.path, err, tt.wantErr)
				return
			}

			if !tt.wantErr {
				if content == nil || len(content) == 0 {
					t.Errorf("GetFile(%q) returned empty content", tt.path)
				}

				if tt.contains != "" && !strings.Contains(string(content), tt.contains) {
					t.Errorf("GetFile(%q) content does not contain %q", tt.path, tt.contains)
				}
			}
		})
	}
}

// TestExtractFile tests extracting individual files
func TestExtractFile(t *testing.T) {
	tests := []struct {
		name       string
		srcPath    string
		targetPath string
		wantErr    bool
		verify     func(t *testing.T, targetPath string)
	}{
		{
			name:       "extract .env.example",
			srcPath:    ".env.example",
			targetPath: "test-output/.env.example",
			wantErr:    false,
			verify: func(t *testing.T, targetPath string) {
				content, err := os.ReadFile(targetPath)
				if err != nil {
					t.Fatalf("Failed to read extracted file: %v", err)
				}
				if !strings.Contains(string(content), "GIT_USER_EMAIL") {
					t.Error("Extracted file missing expected content")
				}
			},
		},
		{
			name:       "extract to nested path",
			srcPath:    "docker-compose.yml",
			targetPath: "test-output/nested/dir/docker-compose.yml",
			wantErr:    false,
			verify: func(t *testing.T, targetPath string) {
				if _, err := os.Stat(targetPath); os.IsNotExist(err) {
					t.Error("Extracted file does not exist")
				}
			},
		},
		{
			name:       "extract non-existent file",
			srcPath:    "nonexistent.txt",
			targetPath: "test-output/nonexistent.txt",
			wantErr:    true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			tmpDir := t.TempDir()
			targetPath := filepath.Join(tmpDir, tt.targetPath)

			err := ExtractFile(tt.srcPath, targetPath)

			if (err != nil) != tt.wantErr {
				t.Errorf("ExtractFile(%q, %q) error = %v, wantErr %v",
					tt.srcPath, targetPath, err, tt.wantErr)
				return
			}

			if !tt.wantErr && tt.verify != nil {
				tt.verify(t, targetPath)
			}
		})
	}
}

// TestExtractDir tests extracting directories recursively
func TestExtractDir(t *testing.T) {
	tests := []struct {
		name      string
		srcDir    string
		targetDir string
		wantErr   bool
		checkFunc func(t *testing.T, targetDir string)
	}{
		{
			name:      "extract scripts directory",
			srcDir:    "scripts",
			targetDir: "extracted-scripts",
			wantErr:   false,
			checkFunc: func(t *testing.T, targetDir string) {
				// Verify directory exists
				if _, err := os.Stat(targetDir); os.IsNotExist(err) {
					t.Error("Target directory does not exist")
				}

				// Verify at least one file was extracted
				entries, err := os.ReadDir(targetDir)
				if err != nil {
					t.Fatalf("Failed to read target directory: %v", err)
				}
				if len(entries) == 0 {
					t.Error("No files extracted from scripts directory")
				}
			},
		},
		{
			name:      "extract templates directory",
			srcDir:    "templates",
			targetDir: "extracted-templates",
			wantErr:   false,
			checkFunc: func(t *testing.T, targetDir string) {
				if _, err := os.Stat(targetDir); os.IsNotExist(err) {
					t.Error("Target directory does not exist")
				}
			},
		},
		{
			name:      "extract docker directory",
			srcDir:    "docker",
			targetDir: "extracted-docker",
			wantErr:   false,
			checkFunc: func(t *testing.T, targetDir string) {
				// Verify Dockerfiles exist
				entries, err := os.ReadDir(targetDir)
				if err != nil {
					t.Fatalf("Failed to read target directory: %v", err)
				}

				foundDockerfile := false
				for _, entry := range entries {
					if strings.HasPrefix(entry.Name(), "Dockerfile.") {
						foundDockerfile = true
						break
					}
				}
				if !foundDockerfile {
					t.Error("No Dockerfile found in extracted docker directory")
				}
			},
		},
		{
			name:      "extract non-existent directory",
			srcDir:    "nonexistent-dir",
			targetDir: "output",
			wantErr:   true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			tmpDir := t.TempDir()
			targetDir := filepath.Join(tmpDir, tt.targetDir)

			err := ExtractDir(tt.srcDir, targetDir)

			if (err != nil) != tt.wantErr {
				t.Errorf("ExtractDir(%q, %q) error = %v, wantErr %v",
					tt.srcDir, targetDir, err, tt.wantErr)
				return
			}

			if !tt.wantErr && tt.checkFunc != nil {
				tt.checkFunc(t, targetDir)
			}
		})
	}
}

// TestExtractAll tests extracting all embedded files
func TestExtractAll(t *testing.T) {
	tmpDir := t.TempDir()

	err := ExtractAll(tmpDir)
	if err != nil {
		t.Fatalf("ExtractAll() error = %v", err)
	}

	// Verify key files exist
	expectedFiles := []string{
		".env.example",
		"docker-compose.yml",
		"backends.py",
		"entrypoint.sh",
		"worker-daemon.py",
		"tools.py",
	}

	for _, file := range expectedFiles {
		path := filepath.Join(tmpDir, file)
		if _, err := os.Stat(path); os.IsNotExist(err) {
			t.Errorf("ExtractAll() did not extract expected file: %s", file)
		}
	}

	// Verify key directories exist
	expectedDirs := []string{
		"docker",
		"scripts",
		"templates",
	}

	for _, dir := range expectedDirs {
		path := filepath.Join(tmpDir, dir)
		info, err := os.Stat(path)
		if os.IsNotExist(err) {
			t.Errorf("ExtractAll() did not extract expected directory: %s", dir)
		} else if err == nil && !info.IsDir() {
			t.Errorf("ExtractAll() extracted %s but it's not a directory", dir)
		}
	}
}

// TestExtractFile_FilePermissions tests that file permissions are preserved
func TestExtractFile_FilePermissions(t *testing.T) {
	tmpDir := t.TempDir()

	// Extract a shell script (should have executable permissions in source)
	targetPath := filepath.Join(tmpDir, "entrypoint.sh")
	err := ExtractFile("entrypoint.sh", targetPath)
	if err != nil {
		t.Fatalf("ExtractFile() error = %v", err)
	}

	// Verify file exists
	info, err := os.Stat(targetPath)
	if err != nil {
		t.Fatalf("Failed to stat extracted file: %v", err)
	}

	// Verify it's readable
	if info.Mode()&0400 == 0 {
		t.Error("Extracted file is not readable")
	}
}

// TestExtractDir_Nested tests extracting nested directory structures
func TestExtractDir_Nested(t *testing.T) {
	tmpDir := t.TempDir()
	targetDir := filepath.Join(tmpDir, "docker-extracted")

	err := ExtractDir("docker", targetDir)
	if err != nil {
		t.Fatalf("ExtractDir() error = %v", err)
	}

	// Verify the directory structure was created
	entries, err := os.ReadDir(targetDir)
	if err != nil {
		t.Fatalf("Failed to read extracted directory: %v", err)
	}

	if len(entries) == 0 {
		t.Error("No files extracted from docker directory")
	}

	// Verify at least one Dockerfile exists
	foundDockerfile := false
	for _, entry := range entries {
		if strings.HasPrefix(entry.Name(), "Dockerfile.") {
			foundDockerfile = true

			// Verify file content
			filePath := filepath.Join(targetDir, entry.Name())
			content, err := os.ReadFile(filePath)
			if err != nil {
				t.Errorf("Failed to read extracted Dockerfile: %v", err)
				continue
			}

			if len(content) == 0 {
				t.Errorf("Extracted Dockerfile %s is empty", entry.Name())
			}
		}
	}

	if !foundDockerfile {
		t.Error("No Dockerfile found in extracted docker directory")
	}
}
