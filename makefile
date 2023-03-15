
astarpa: cpu-freq prep
	cargo build --release --no-default-features
	cargo run --release -- --release evals/astarpa/experiments/*.yaml --cache evals/astarpa/results/cache.json

cpu-freq:
	sudo cpupower frequency-set -d 2.6GHz -u 2.6GHz -g powersave

prep:
	systemctl --user stop emacs || true
	kill chromium || true
	kill telegram-desktop || true
	kill signal-desktop || true
	kill slack || true
