# Maintainer: Your Name <your.email@example.com>
pkgname=sensors-monitor
pkgver=1.0.0
pkgrel=1
pkgdesc="A terminal-based system sensors monitor with color-coded output"
arch=('any')
url="https://github.com/bobi/sensors-monitor"
license=('MIT')
depends=('lm_sensors' 'python-rich')
source=("sensors-monitor.py" "sensors-monitor.conf")
sha256sums=('SKIP' 'SKIP')

package() {
    install -Dm755 "$srcdir/sensors-monitor.py" "$pkgdir/usr/bin/sensors-monitor"
    install -Dm644 "$srcdir/sensors-monitor.conf" "$pkgdir/etc/sensors-monitor.example.conf"
}
