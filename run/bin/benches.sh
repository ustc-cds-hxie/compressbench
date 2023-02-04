#!/bin/bash

BASE=`pwd`/..

BENCH_DATA="$BASE/benchdata"
BENCH_RESULTS="$BASE/benchresults"

FILE_TO_COMPRESS="$1"
[ -z "$FILE_TO_COMPRESS" ] && FILE_TO_COMPRESS="$BENCH_DATA/hackatom.wasm.serialized"
export FILE_TO_COMPRESS

cargo bench | tee "$BENCH_RESULTS/benches.txt"
