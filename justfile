set windows-shell := ["pwsh", "-NoLogo", "-Command"]

svgo-config := '''
export default {
  plugins: [
    { name: "removeAttrs", params: { attrs: "class" } },
    "preset-default",
    "removeViewBox",
  ],
  js2svg: { indent: 2, pretty: true },
}
'''

default: fmt build test

[unix]
build:
    cd plugin && RUSTFLAGS="-Zunstable-options -Cpanic=immediate-abort" cargo +nightly -Zbuild-std=std build --release --target wasm32-unknown-unknown
    wasm-opt --enable-bulk-memory-opt -Oz -o src/zebra.wasm plugin/target/wasm32-unknown-unknown/release/typst_zebra.wasm

    typst compile --root . examples/readme-header.typ -f svg
    typst compile --root . examples/readme-optimized-path.typ -f svg
    @sed -i '' '1s/\([0-9]\)pt"/\1"/g' examples/readme-*.svg
    @echo '{{svgo-config}}' > svgo.config.js
    svgo examples/readme-*.svg
    @rm svgo.config.js

[windows]
build:
    cd plugin; $env:RUSTFLAGS = "-Zunstable-options -Cpanic=immediate-abort"; cargo +nightly -Zbuild-std=std build --release --target wasm32-unknown-unknown
    wasm-opt --enable-bulk-memory-opt --enable-nontrapping-float-to-int -Oz -o src/zebra.wasm plugin/target/wasm32-unknown-unknown/release/typst_zebra.wasm

    typst compile --root . examples/readme-header.typ -f svg
    typst compile --root . examples/readme-optimized-path.typ -f svg
    @Get-ChildItem examples/readme-*.svg | ForEach-Object { (Get-Content $_ -Raw) -replace '(\d)pt"', '$1"' | Set-Content $_ }
    @'{{ svgo-config }}' | Set-Content svgo.config.js
    @Get-ChildItem examples/readme-*.svg | ForEach-Object { svgo $_.FullName }
    @Remove-Item svgo.config.js

fmt:
    cd plugin && cargo fmt
    typstyle --line-width 100 -i src examples

test:
    cd plugin && cargo test
    typst compile --root . examples/tests.typ -f svg
    typst compile --root . examples/test-readme.typ -f svg

alias w := watch
watch typst-file="tests":
    typst watch --root . '{{ if typst-file =~ "\\.typ$" { typst-file } else { "examples/" + typst-file + ".typ" } }}' -f svg
