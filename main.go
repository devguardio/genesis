package main

import (
	"archive/tar"
	"compress/gzip"
	"fmt"
	"io"
	"io/ioutil"
	"os"
	"path/filepath"
	"strings"

	"github.com/pelletier/go-toml"
)

const genesisPath = "etc/config/genesis"

func main() {
	args := os.Args[1:]
	currentconfig := filepath.Join(genesisPath, "current.toml")
	if len(args) == 0 {
		println("Need at least one argument \"settle\" or \"revert\"")
	}
	if len(args) == 2 {
		currentconfig = args[1]
	}

	if args[0] == "settle" {
		stabilize(currentconfig)
	}
	if args[0] == "revert" {
		revert(currentconfig)
	}
	if args[0] == "apply" {
		genesisCommit(currentconfig)
	}
}

func genesisCommit(current string) {
	config, err := loadCurrentFile(current)
	if err != nil {
		fmt.Printf("Failed to load configuration file: %v\n", err)
		return
	}
	em := newEmitter()
	err = em.load(config)
	if err != nil {
		fmt.Printf("Failed to load configuration file: %v\n", err)
	}
	err = em.commit()
	if err != nil {
		fmt.Printf("Failed to commit file: %v\n", err)
	}
}

func stabilize(current string) {
	println("genesis stabilized")

	err := os.Rename(current, filepath.Join(genesisPath, "stable.toml"))
	if err != nil {
		fmt.Printf("genesis: %v", err)
	}
}

func revert(current string) {
	println("genesis reverting")
	err := os.Remove(current)
	if err != nil {
		fmt.Printf("genesis: %v", err)
	}
	genesisCommit(current)
}

func loadCurrentFile(current string) (genesis, error) {
	fpath := current
	_, err := os.Stat(fpath)
	if os.IsNotExist(err) {
		fpath = filepath.Join(genesisPath, "stable.toml")
	}

	config, err := ioutil.ReadFile(fpath)
	if err != nil {
		return genesis{}, err
	}
	gen := genesis{
		Interfaces: make(map[string]devInterface),
		Devices:    make(map[string]device),
		Templates:  make(map[string]template),
	}
	err = toml.Unmarshal(config, &gen)
	if err != nil {
		return genesis{}, err
	}
	return gen, nil
}

func compress(src string, files ...string) error {
	outputFileName := filepath.Base(src)
	out, err := os.Create(outputFileName + ".tar.gz")
	if err != nil {
		return err
	}
	zr := gzip.NewWriter(out)
	tw := tar.NewWriter(zr)

	filepath.Walk(src, func(file string, fi os.FileInfo, err error) error {
		if !containsPrefix(files, filepath.Clean(file)) {
			if fi.IsDir() {
				return filepath.SkipDir
			}
			return nil
		}
		header, err := tar.FileInfoHeader(fi, file)
		if err != nil {
			return err
		}

		//https://golang.org/src/archive/tar/common.go?#L626
		header.Name = filepath.ToSlash(file)

		if err := tw.WriteHeader(header); err != nil {
			return err
		}
		//skip dirs
		if !fi.IsDir() {
			data, err := os.Open(file)
			if err != nil {
				return err
			}
			if _, err := io.Copy(tw, data); err != nil {
				return err
			}
		}
		return nil
	})
	if backend == "openwrt" {
		file, err := os.Open("etc/devguard/genesis/post")
		if err != nil {
			return err
		}
		fi, err := file.Stat()
		if err != nil {
			return err
		}

		header, err := tar.FileInfoHeader(fi, "uci.defaults")
		if err != nil {
			return err
		}
		header.Name = filepath.ToSlash("uci.defaults")

		if err := tw.WriteHeader(header); err != nil {
			return err
		}

		if _, err := io.Copy(tw, file); err != nil {
			return err
		}
	}
	err = tw.Close()
	if err != nil {
		return err
	}
	err = zr.Close()
	if err != nil {
		return err
	}
	return nil
}

func containsPrefix(s []string, p string) bool {
	for _, str := range s {
		if strings.HasPrefix(str, p) {
			return true
		}
	}
	return false
}
