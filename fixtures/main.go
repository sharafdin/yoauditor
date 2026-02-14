// Intentional issues: defer in loop (resource leak), error ignored

package main

import "os"

func processFiles(paths []string) {
	for _, path := range paths {
		f, err := os.Open(path)
		if err != nil {
			continue // Error silently ignored
		}
		// Defer in loop: file not closed until function returns - leak
		defer f.Close()
		// ... use f
	}
}

func getEnv(key string) string {
	// No validation: empty or missing env could cause issues downstream
	return os.Getenv(key)
}
