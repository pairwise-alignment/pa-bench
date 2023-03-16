run:
	cargo build --release --no-default-features
	cd ../../ && cargo run --release -- --release experiments/*.yaml --cache results/cache.json

run_release: cpu-freq prep run

export_plots:
	cp -r plots ../../../../pairwise-aligner-paper/plots

cpu-freq:
	sudo cpupower frequency-set -d 2.6GHz -u 2.6GHz -g powersave

# Kill some background programs.
prep:
	systemctl --user stop emacs || true
	/usr/bin/kill signal-desktop || true
	/usr/bin/kill chromium || true
	/usr/bin/kill telegram-desktop || true
	/usr/bin/kill slack || true

zip_results:
	zip results.zip results/*.json
unzip_results:
	unzip results.zip
