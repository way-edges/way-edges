# This is an example PKGBUILD file. Use this as a start to creating your own,
# and remove these comments. For more information, see 'man PKGBUILD'.
# NOTE: Please fill out the license field for your package! If it is unknown,
# then please put 'unknown'.

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
source=("$_pkgname::git+$url")
sha256sums=('SKIP')

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
