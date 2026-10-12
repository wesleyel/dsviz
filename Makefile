# dsviz development tasks. Run `make help` for a list.

VERSION := $(shell sed -n 's/^version *= *"\(.*\)"/\1/p' typst.toml)
WASM    := plugin/target/wasm32-unknown-unknown/release/dsviz.wasm
PKGDIR  := build/pkg

.PHONY: help wasm test gallery examples images package check clean

help:
	@echo "make wasm      build the plugin and copy it to src/dsviz.wasm"
	@echo "make test      Rust unit tests + compile the gallery, README snippets and examples"
	@echo "make images    regenerate README images in docs/images/"
	@echo "make package   stage the publishable bundle in $(PKGDIR)/preview/dsviz/$(VERSION)"
	@echo "make check     verify versions are consistent across files"
	@echo "scripts/bump.sh X.Y.Z   bump the version everywhere"

wasm:
	cd plugin && cargo build --release --target wasm32-unknown-unknown
	cp $(WASM) src/dsviz.wasm
	@if command -v wasm-opt >/dev/null 2>&1; then \
		wasm-opt -Oz --enable-bulk-memory src/dsviz.wasm -o src/dsviz.wasm; \
	else echo "note: wasm-opt not found, skipping size optimisation"; fi
	@ls -l src/dsviz.wasm

test: gallery examples
	cd plugin && cargo test --quiet

gallery:
	@mkdir -p build
	typst compile --root . tests/gallery.typ build/gallery.pdf

# Examples import "@preview/dsviz:<version>", served from the staged bundle.
examples: package
	@mkdir -p build/examples
	@for f in examples/*.typ tests/readme.typ; do \
		echo "compile $$f"; \
		typst compile --root . --package-path $(PKGDIR) $$f build/examples/$$(basename $$f .typ).pdf || exit 1; \
	done

images: package
	@for f in examples/*.typ; do \
		n=$$(basename $$f .typ); \
		typst compile --root . --package-path $(PKGDIR) --ppi 200 $$f docs/images/$$n.png || exit 1; \
	done
	@if command -v oxipng >/dev/null 2>&1; then oxipng -q -o 4 --strip safe docs/images/*.png; fi
	@ls -l docs/images

package:
	@scripts/package.sh $(PKGDIR)/preview

check:
	@scripts/check-version.sh

clean:
	rm -rf build
