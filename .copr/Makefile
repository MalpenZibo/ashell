srpm:
	dnf -y install git rust-srpm-macros rpmautospec-rpm-macros rpmdevtools
	$(eval VERSION := $(shell grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/'))
	spectool -g .rpm/ashell.spec -C . --define "pkg_version $(VERSION)"
	rpmbuild -bs .rpm/ashell.spec \
		--define "pkg_version $(VERSION)" \
		--define "_sourcedir $(PWD)" \
		--define "_srcrpmdir $(outdir)"
