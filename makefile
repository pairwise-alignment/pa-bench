default: astarpa

cpu-freq:
	sudo cpupower frequency-set -d 2.6GHz -u 2.6GHz -g powersave

prep:
	systemctl --user stop emacs || true
	/usr/bin/kill signal-desktop || true
	/usr/bin/kill chromium || true
	/usr/bin/kill telegram-desktop || true
	/usr/bin/kill slack || true

astarpa:
	cargo build --release --no-default-features
	cargo run --release -- --release evals/astarpa/experiments/*.yaml --cache evals/astarpa/results/cache.json

astarpa_release: cpu-freq prep astarpa

astarpa_export:
	cp evals/astarpa/plots/* ../../pairwise-aligner-paper/imgs/
