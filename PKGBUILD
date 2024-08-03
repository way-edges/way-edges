# Maintainer: OGIOS <ogios@foxmail.com>
_pkgname=way-edges
pkgname=way-edges-git
pkgver=0.1
pkgrel=1
pkgdesc="Hidden widget on screen edges"
arch=('x86_64' 'aarch64')
url="https://github.com/ogios/way-edges"
license=('MIT')
depends=('gtk4' 'gtk4-layer-shell' 'cairo' 'pango')
makedepends=(cargo git)
provides=(way-edges)
options=(!debug)
# source=("$_pkgname::git+$url")
# sha256sums=('SKIP')

prepare() {
  git clone "$url.git" "$_pkgname" --depth=1
}

pkgver() {
  cd "$_pkgname"
  printf "r%s.%s" "$(git rev-list --count HEAD)" "$(git rev-parse --short HEAD)"
}

build() {
  cd "$_pkgname"
  cargo build --release
}

package() {
  cd "$_pkgname"
  install -Dm755 "target/release/$_pkgname" "$pkgdir/usr/bin/$_pkgname"
  install -Dm644 "LICENSE" "$pkgdir/usr/share/licenses/$pkgname/LICENCE"
}
