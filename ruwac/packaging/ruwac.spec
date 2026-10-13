# Upstream testing RPMs use the version already assigned by the release controller.
%global debug_package %{nil}
%bcond_without desktop
Name: ruwac
Version: %{ruwac_version}
Release: 1%{?dist}
Summary: Persistent terminal daemon and command-line workspace client
License: %{ruwac_core_license}
URL: https://github.com/w3fans/RUWAC
Source0: ruwac-testing-payload.tar.gz
ExclusiveArch: x86_64
Requires: python3
Requires: openssh-clients

%description
Persistent daemon-owned terminals and authenticated command-line control.
This upstream Fedora testing package preserves existing sessions and configuration.
It is separate from the optional GTK desktop and does not autostart services.

%if %{with desktop}
%package desktop
Summary: Native GTK remote workspace and RDP client
License: %{ruwac_desktop_license}
Requires: ruwac = %{version}-%{release}
Requires: gtk4
Requires: libadwaita
Requires: openssl-libs
Requires: libssh2
Requires: sqlite-libs
Requires: python3-pillow
Requires: tesseract
Requires: tesseract-langpack-eng

%description desktop
Native GTK workspace, authenticated RDP, desktop clipboard and file transfers.
This is an upstream testing candidate, not a Fedora repository qualification.
The known desktop RSA advisory remains unresolved; see the installed testing notes.
%endif

%prep
%setup -q -n ruwac-rpm-payload

%build
# Payloads are compiled and tested from the controller's source by build.sh.
# This packaging adapter neither recompiles nor assigns a product version.

%install
install -D -m 0755 target/release/ruwac %{buildroot}%{_bindir}/ruwac
install -D -m 0644 source.json %{buildroot}%{_datadir}/ruwac/rpm-installation.json
%if %{with desktop}
install -D -m 0755 target/release/ruwac-desktop %{buildroot}%{_bindir}/ruwac-desktop
install -D -m 0644 ruwac.desktop %{buildroot}%{_datadir}/applications/si.z4.ruwac.desktop
%endif

%files
%license LICENSE notices/core
%doc TESTING.txt
%{_bindir}/ruwac
%dir %{_datadir}/ruwac
%{_datadir}/ruwac/rpm-installation.json

%if %{with desktop}
%files desktop
%license LICENSE notices/desktop
%doc TESTING.txt
%{_bindir}/ruwac-desktop
%{_datadir}/applications/si.z4.ruwac.desktop
%endif
