#!/usr/bin/env bash
# Freezes the rolling docs as a release: copies content/docs/rolling to
# content/docs/<version>, points its internal links at the copy, and lists
# the version in versions.toml as the newest release, which the docs then
# open on, /docs/ included.
#
#   scripts/freeze-version.sh 0.1.0
set -euo pipefail

version="${1:?usage: $0 <version>}"
site="$(cd "$(dirname "$0")/.." && pwd)"
docs="$site/content/docs"

if [[ ! "$version" =~ ^[0-9A-Za-z][0-9A-Za-z._-]*$ || "$version" == "rolling" ]]; then
  echo "not a version name: $version" >&2
  exit 1
fi
if [[ -e "$docs/$version" ]]; then
  echo "content/docs/$version exists already" >&2
  exit 1
fi

cp -r "$docs/rolling" "$docs/$version"
find "$docs/$version" -name '*.md' -exec sed -i "s|@/docs/rolling/|@/docs/$version/|g" {} +

# Right after the rolling entry, so the releases stay newest first.
awk -v name="$version" '
  { print }
  /^kind = "rolling"/ && !done {
    print ""
    print "[[versions]]"
    print "name = \"" name "\""
    print "label = \"" name "\""
    print "kind = \"release\""
    done = 1
  }
' "$site/versions.toml" > "$site/versions.toml.new"
mv "$site/versions.toml.new" "$site/versions.toml"

sed -i "s|^redirect_to = .*|redirect_to = \"docs/$version/\"|" "$docs/_index.md"

echo "Froze the rolling docs as $version."
