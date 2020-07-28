package main

import (
	"bytes"
	"io/ioutil"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestEverything(t *testing.T) {
	files, _ := ioutil.ReadDir("tests")
	for _, file := range files {
		if !strings.HasSuffix(file.Name(), ".toml") {
			continue
		}
		if backend == "openwrt" {
			testWRT(t, strings.TrimSuffix(file.Name(), ".toml"))
		} else if backend == "systemd" {
			testSystemd(t, strings.TrimSuffix(file.Name(), ".toml"))
		}
	}
}

func testWRT(t *testing.T, name string) {
	defer os.RemoveAll("etc")
	genesisCommit("tests/" + name + ".toml")
	filepath.Walk("etc", VisitFile(t, name))
}

func testSystemd(t *testing.T, name string) {
	defer os.RemoveAll("etc")
	genesisCommit("tests/" + name + ".toml")
	filepath.Walk("etc", VisitFile(t, name))
}

func VisitFile(t *testing.T, name string) func(fp string, fi os.FileInfo, err error) error {
	return func(fp string, fi os.FileInfo, err error) error {
		if err != nil {
			return err
		}
		if fi.IsDir() {
			return nil
		}
		fileEQ(t, filepath.Join("tests", name, fp), fp)
		return nil
	}
}

func fileEQ(t *testing.T, original, b string) bool {
	out, err := ioutil.ReadFile(original)
	if err != nil {
		t.Fatal(err)
		return false
	}
	res, err := ioutil.ReadFile(b)
	if err != nil {
		t.Fatal(err)
		return false
	}
	if bytes.Compare(out, res) != 0 {
		t.Error(b + " not the same as " + original)
		return false
	}
	return true
}
