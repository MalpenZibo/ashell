%define __spec_install_post %{nil}
%define __os_install_post %{_dbpath}/brp-compress
%define debug_package %{nil}

Name: ashell
Summary: A ready to go status bar for Wayland compositors
Version: @@VERSION@@
Release: @@RELEASE@@%{?dist}
License: GPL-3.0-or-later
Source0: %{name}-%{version}.tar.gz
URL: https://github.com/MalpenZibo/ashell

%description
%{summary}

%prep
%setup -q

%install
rm -rf %{buildroot}
mkdir -p %{buildroot}
cp -a * %{buildroot}

%clean
rm -rf %{buildroot}

%files
%license %{_datadir}/licenses/ashell/LICENSE
%{_bindir}/*
