#!/bin/bash

# Maximum desired compression time
MAX_COMPRESSION_TIME="100000000" # (100 ms) [ns]
# Maximum desired decompression time
MAX_DECOMPRESSION_TIME="100000" # (100 us) [ns]

# For proper numerical sort
LC_ALL=C
export LC_ALL

BASE=`dirname $0`/..
BENCH_RESULTS="$BASE/benchresults"

# Filter/organize fields: bench,time,size
egrep '(^compression\b|\btime:|\bbytes\b|^Benchmarking\b)' "$BENCH_RESULTS/benches.txt" | grep -v '^uncompressed:' | grep -v '(p = ' | sed 's/^Benchmarking\b.*//' | sed '/^compression\b/{N;N;s/\n/ /g}' | grep -v '^$' | awk '{print $1","$5" "$6","$11}' >"$BENCH_RESULTS/benches.csv"

# Uncompressed file size
USZ=`grep '^uncompressed_u8:' "$BENCH_RESULTS/benches.txt" | cut -d\  -f4`
(
echo "bench,time,size,compression_ratio,ns,time_ratio,quality"
while read L
do
	# Compute nanoseconds
	NS=`echo $L | cut -f2 -d, | sed 's/ ns$//;s/ us$/\*1000/;s/ ms$/*1000000/;s/ s$/*1000000000/' | bc -l`

  # Compute compression ratio
	C=0
	SZ=`echo $L | cut -f3 -d,`
	[ -n "$SZ" ] && C=`echo -e "scale=3\n$USZ/$SZ" | bc -l`

  # Compute (adjusted) time ratio
	MAX_TIME=$MAX_COMPRESSION_TIME
	QN=2
	if echo $L | cut -d, -f1 | fgrep -q 'unpack'
	then
		MAX_TIME=$MAX_DECOMPRESSION_TIME
		QN=1
	fi
	T=`echo -e "scale=10\n$NS/$MAX_TIME-1." | bc -l`

	# Compute quality
	Q=`echo -e "scale=10\n1.-($C+$T)/$QN" | bc -l`

	echo "$L,$C,$NS,$T,$Q"
done <"$BENCH_RESULTS/benches.csv" | sort -t, -nrk7
) >"$BENCH_RESULTS/benches-2.csv"
# mv "$BENCH_RESULTS/benches-2.csv" "$BENCH_RESULTS/benches.csv"

# Pack and unpack
egrep '(^bench,|\/unpack)' "$BENCH_RESULTS/benches-2.csv" >"$BENCH_RESULTS/benches-unpack.csv"
egrep '(^bench,|\/pack)' "$BENCH_RESULTS/benches-2.csv" >"$BENCH_RESULTS/benches-pack.csv"

# Total (pack+unpack)
(
echo "bench,pack_time,unpack_time,compression_ratio,total_quality"
egrep -v '^bench\b' "$BENCH_RESULTS/benches.csv" | sort -t, -k1 | sed 'N;s/\n/,/' | awk -F, '{print $1","$2","$9","$4","($7+$14)/2.}' | sed 's/.pack,/,/' | sort -t, -nrk5
) | tee "$BENCH_RESULTS/benches-total.csv"
