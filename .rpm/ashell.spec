%define __spec_install_post %{nil}
%define __os_install_post %{_dbpath}/brp-compress
%define debug_package %{nil}

%global rust_version 1.89

Name: ashell
Summary: A ready to go Wayland status bar for Hyprland and Niri
Version: @@VERSION@@
Release: @@RELEASE@@%{?dist}
License: GPL-3.0-or-later
URL: https://github.com/MalpenZibo/ashell
Source0: %{name}-%{version}.tar.gz

BuildRequires:  cargo
BuildRequires:  rust >= %{rust_version}
BuildRequires:  gcc
BuildRequires:  make
BuildRequires:  pkgconf-pkg-config
BuildRequires:  pipewire-devel
BuildRequires:  libinput-devel
BuildRequires:  systemd-devel
BuildRequires:  dbus-devel
BuildRequires:  wayland-devel
BuildRequires:  libxkbcommon-devel
BuildRequires:  mesa-libEGL-devel
BuildRequires:  pango-devel
BuildRequires:  cairo-devel
BuildRequires:  glib2-devel
BuildRequires:  openssl-devel
BuildRequires:  fontconfig-devel
BuildRequires:  freetype-devel
BuildRequires:  clang-devel
BuildRequires:  llvm-devel
BuildRequires:  pulseaudio-libs-devel
BuildRequires:  systemd-rpm-macros

Requires:       libwayland-client
Requires:       pipewire-libs
Requires:       pulseaudio-libs

%description
A ready to go Wayland status bar for Hyprland and Niri.

%prep
%setup -q

%build
cargo build --release

%install
install -Dm755 target/release/ashell %{buildroot}%{_bindir}/ashell

%check
test -x %{buildroot}%{_bindir}/ashell

%clean
rm -rf %{buildroot}

%files
%{_bindir}/ashell

%license LICENSE
%doc README.md

%changelog
* Mon Jun 22 2026 Simone Camito <scamito@outlook.it> - @@VERSION@@-@@RELEASE@@
- Initial COPR build
