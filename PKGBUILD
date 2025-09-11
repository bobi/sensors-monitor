# Maintainer: Your Name <your.email@example.com>
pkgname=sensors-monitor
pkgver=1.1.0
pkgrel=1
pkgdesc="A terminal-based system sensors monitor with color-coded output"
arch=('any')
url="https://github.com/bobi/sensors-monitor"
license=('MIT')
depends=('lm_sensors')
makedepends=(cargo)
source=()
# sha256sums=('SKIP' 'SKIP')

# prepare() {
# 	ln -snf "$startdir" "$srcdir/$pkgname"
# }

build() {
# cd "$srcdir/$pkgname-$pkgver"
    cd "${startdir}"
    cargo build --release
}

package() {
# 	cd "${pkgname}-${pkgver}"
    cd "${startdir}"
	install -vDm755 "target/release/${pkgname}" "${pkgdir}/usr/bin/${pkgname}"
	install -Dm644 "sensors-monitor.conf" "${pkgdir}/usr/share/${pkgname}/sensors-monitor.conf"
# 	install -vDm644 -t "${pkgdir}/usr/share/licenses/${pkgname}" LICENSE
# 	install -vDm644 -t "${pkgdir}/usr/share/doc/${pkgname}" README.md
}
