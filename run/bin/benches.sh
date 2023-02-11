#!/bin/bash

BASE=`pwd`/..

BENCH_DATA="$BASE/benchdata"
BENCH_RESULTS="$BASE/benchresults"

FILE_TO_COMPRESS="$1"
[ -z "$FILE_TO_COMPRESS" ] && FILE_TO_COMPRESS="$BENCH_DATA/devinrsmith-air-quality.20220714.zstd.parquet"
export FILE_TO_COMPRESS

cargo bench | tee "$BENCH_RESULTS/benches.txt"
