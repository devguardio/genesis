package main

import (
	"bytes"
	"fmt"
	"io/ioutil"
	"os"
	"path/filepath"

	"github.com/pelletier/go-toml"
)

const genesisPath = "genesis"

func main() {
	args := os.Args[1:]
	fmt.Println(args)
	currentconfig := "current.toml"
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
	em := emitter{
		devices:    make(map[string]struct{}, 0),
		interfaces: make(map[string]netInterface, 0),
		wireless:   &bytes.Buffer{},
		network:    &bytes.Buffer{},
		tplout:     make(map[string]string, 0),
	}
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

	err := os.Rename(filepath.Join(genesisPath, current), filepath.Join(genesisPath, "stable.toml"))
	if err != nil {
		fmt.Printf("genesis: %v", err)
	}
}

func revert(current string) {
	println("genesis reverting")
	err := os.Remove(filepath.Join(genesisPath, current))
	if err != nil {
		fmt.Printf("genesis: %v", err)
	}
	genesisCommit(current)
}

func loadCurrentFile(current string) (Genesis, error) {
	fpath := filepath.Join(genesisPath, current)
	_, err := os.Stat(fpath)
	if os.IsNotExist(err) {
		fpath = filepath.Join(genesisPath, "stable.toml")
	}

	config, err := ioutil.ReadFile(fpath)
	if err != nil {
		return Genesis{}, err
	}
	gen := Genesis{
		Interfaces: make(map[string]DevInterface),
		Devices:    make(map[string]Device),
		Templates:  make(map[string]Template),
	}
	err = toml.Unmarshal(config, &gen)
	if err != nil {
		return Genesis{}, err
	}
	return gen, nil
}
