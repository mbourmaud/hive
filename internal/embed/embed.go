package embed

import (
	"embed"
	"io/fs"
	"os"
	"path/filepath"
)

//go:embed all:files
var Files embed.FS

// GetFile reads a file from the embedded filesystem
func GetFile(path string) ([]byte, error) {
	return Files.ReadFile("files/" + path)
}

// ExtractAll extracts all embedded files to the target directory
func ExtractAll(targetDir string) error {
	return fs.WalkDir(Files, "files", func(path string, d fs.DirEntry, err error) error {
		if err != nil {
			return err
		}

		// Skip the root "files" directory
		if path == "files" {
			return nil
		}

		// Get relative path (remove "files/" prefix)
		relPath, _ := filepath.Rel("files", path)
		targetPath := filepath.Join(targetDir, relPath)

		if d.IsDir() {
			return os.MkdirAll(targetPath, 0755)
		}

		// Read file content
		content, err := Files.ReadFile(path)
		if err != nil {
			return err
		}

		// Get file info for permissions
		info, err := d.Info()
		if err != nil {
			return err
		}

		// Create parent directory if needed
		if err := os.MkdirAll(filepath.Dir(targetPath), 0755); err != nil {
			return err
		}

		// Write file with same permissions
		return os.WriteFile(targetPath, content, info.Mode())
	})
}

// ExtractFile extracts a single file to the target path
func ExtractFile(srcPath, targetPath string) error {
	content, err := GetFile(srcPath)
	if err != nil {
		return err
	}

	// Create parent directory if needed
	if err := os.MkdirAll(filepath.Dir(targetPath), 0755); err != nil {
		return err
	}

	return os.WriteFile(targetPath, content, 0644)
}

// ExtractDir extracts a directory recursively to the target path
func ExtractDir(srcDir, targetDir string) error {
	srcPath := "files/" + srcDir

	return fs.WalkDir(Files, srcPath, func(path string, d fs.DirEntry, err error) error {
		if err != nil {
			return err
		}

		// Get relative path from srcPath
		relPath, _ := filepath.Rel(srcPath, path)
		if relPath == "." {
			return os.MkdirAll(targetDir, 0755)
		}

		targetPath := filepath.Join(targetDir, relPath)

		if d.IsDir() {
			return os.MkdirAll(targetPath, 0755)
		}

		// Read and write file
		content, err := Files.ReadFile(path)
		if err != nil {
			return err
		}

		info, err := d.Info()
		if err != nil {
			return err
		}

		return os.WriteFile(targetPath, content, info.Mode())
	})
}
