#!/usr/bin/env bash

set -ex

cargo run --release -- "$@"
