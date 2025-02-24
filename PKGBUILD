# Maintainer: Your Name <your.email@example.com>
pkgname=sensors-monitor
pkgver=1.0.0
pkgrel=1
pkgdesc="A terminal-based system sensors monitor with color-coded output"
arch=('any')
url="https://github.com/bobi/sensors-monitor"
license=('MIT')
depends=('lm_sensors' 'python-rich')
makedepends=(python-{build,installer,setuptools,wheel})
source=()
# sha256sums=('SKIP' 'SKIP')

# prepare() {
# 	ln -snf "$startdir" "$srcdir/$pkgname"
# }

build() {
# 	cd "${pkgname}-${pkgver}"
#     cd "$pkgname"
    cd "$startdir"
	python -m build --wheel --no-isolation
}

package() {
# 	cd "${pkgname}-${pkgver}"
    cd "$startdir"
	python -m installer --destdir="${pkgdir}" dist/*.whl
# 	install -vDm644 -t "${pkgdir}/usr/share/licenses/${pkgname}" LICENSE
# 	install -vDm644 -t "${pkgdir}/usr/share/doc/${pkgname}" README.md
}
