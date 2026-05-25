Name:           ashell
Version:        %{pkg_version}
Release:        %autorelease
Summary:        A ready to go Wayland status bar for Hyprland and Niri

SourceLicense:  MIT
# FIXME: paste output of %%cargo_license_summary here
License:        %{shrink:
    MIT AND
    TODO
}
# LICENSE.dependencies contains a full license breakdown

URL:            https://github.com/MalpenZibo/ashell
Source:         %{url}/archive/v%{version}/ashell-%{version}.tar.gz

BuildRequires:  cargo-rpm-macros
BuildRequires:  pkgconf-pkg-config
BuildRequires:  pipewire-devel
BuildRequires:  libinput-devel
BuildRequires:  systemd-devel
BuildRequires:  dbus-devel
BuildRequires:  wayland-devel
BuildRequires:  libxkbcommon-devel
BuildRequires:  mesa-libEGL-devel
BuildRequires:  openssl-devel
BuildRequires:  clang-devel

%description
ashell is a ready to go Wayland status bar for Hyprland and Niri compositors.
It supports multi-monitor, hot-reload configuration, theming, system tray,
notifications, media player, and many more features.

%prep
%autosetup -n ashell-%{version} -p1
%cargo_prep

%generate_buildrequires
%cargo_generate_buildrequires

%build
%cargo_build
%{cargo_license_summary}
%{cargo_license} > LICENSE.dependencies

%install
install -Dpm 0755 target/rpm/ashell -t %{buildroot}%{_bindir}

%files
%license LICENSE
%license LICENSE.dependencies
%doc README.md
%{_bindir}/ashell

%changelog
%autochangelog
