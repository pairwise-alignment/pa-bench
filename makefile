cpu-freq:
	sudo cpupower frequency-set -d 2.6GHz -u 2.6GHz -g powersave
astarpa: cpu-freq
	for e in evals/experiments/astarpa/* ; do cargo run --release -- --nice=-20 -j5 -i $$e ; done
