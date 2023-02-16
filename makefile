cpu-freq:
	sudo cpupower frequency-set -d 2.6GHz -u 2.6GHz -g powersave
astarpa: cpu-freq
	cargo build --release --no-default-features
	cargo run --release -- --nice=-20 -q evals/experiments/astarpa/*.yaml
affine: cpu-freq
	cargo build --release --no-default-features
	cargo run --release -- --nice=-20 -q evals/experiments/affine_cost_scaling.yaml
