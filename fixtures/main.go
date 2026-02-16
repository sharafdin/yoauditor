package main

import "os"

func processFiles(paths []string) {
	for _, path := range paths {
		f, err := os.Open(path)
		if err != nil {
			continue
		}
		defer f.Close()
	}
}

func getEnv(key string) string {
	return os.Getenv(key)
}
