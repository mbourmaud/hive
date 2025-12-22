package main

import (
	"os"

	"github.com/mbourmaud/hive/cmd"
)

func main() {
	if err := cmd.Execute(); err != nil {
		os.Exit(1)
	}
}
