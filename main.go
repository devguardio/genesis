package main

import (
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
