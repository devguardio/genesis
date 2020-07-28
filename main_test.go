package main

import (
	"bytes"
	"io/ioutil"
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
	genesisCommit("tests/" + name + ".toml")
	out, err := ioutil.ReadFile("./etc/config/network")
	if err != nil {
		t.Fatal("failled to open network output file")
	}
	res, err := ioutil.ReadFile("tests/" + name + ".network")
	if err != nil {
		t.Fatal("no expected network file")
	}
	if bytes.Compare(out, res) != 0 {
		t.Error("network file not the same")
	}
	out, err = ioutil.ReadFile("./etc/config/wireless")
	if err != nil {
		t.Fatal("failled to open wireless output file")
	}
	res, err = ioutil.ReadFile("tests/" + name + ".wireless")
	if err != nil {
		t.Fatal("no expected wireless file")
	}
	if bytes.Compare(out, res) != 0 {
		t.Error("wireless file not the same")
	}
}

func testSystemd(t *testing.T, name string) {
	genesisCommit("tests/" + name + ".toml")
	files, _ := ioutil.ReadDir("tests")
	for _, file := range files {
		if !strings.HasPrefix(file.Name(), name+".d.") {
			continue
		}
		of := "./etc/systemd/network/20-genesis-" + strings.TrimPrefix(file.Name(), name+".d.")
		if !fileEQ(t, "tests/"+file.Name(), of) {
			t.Fail()
		}
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
