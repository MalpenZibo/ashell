# website

The site of ashell-experimental: a landing page, a gallery and the docs,
built with [Zola](https://www.getzola.org) 0.23. It is served under the stable
site, at `https://malpenzibo.github.io/ashell/experimental/`: the Pages
workflow builds it into the stable site's build, as it does the developer
guide.

```bash
cd experimental/website
zola serve    # http://127.0.0.1:1111, rebuilt on save
zola build    # into public/
```

## Layout

| Path | What it holds |
| ---- | ------------- |
| `content/_index.md` | The landing page: its tagline and features are in the front matter. |
| `content/gallery/` | The gallery: every image in the folder, by file name, with its caption from the front matter. The landing page shows the first four. |
| `content/docs/<version>/` | The docs of one version. `rolling` follows `main`. |
| `versions.toml` | The versions, as the version switcher lists them. |
| `templates/components.html` | The components the templates and the docs use, as `note`. |
| `sass/style.scss` | The one stylesheet. |
| `static/search.js` | The search: Zola builds the index (`search_index.en.json`) and ships elasticlunr; it searches the docs of the version being read. Open it with the navbar button, `/` or Ctrl+K. |

## The docs

Pages are Markdown, ordered by `weight`. A page or section can set
`extra.sidebar`, its name in the sidebar when it differs from its title.
A box in a page is the `note` component:

```md
{% <note kind="warning" title="Experimental"> %}
There are no releases: it is built from `main`.
{% </note> %}
```

`kind` is `info`, `tip` or `warning`. Link to another page by its file, as
`[Examples](@/docs/rolling/examples.md)`: Zola checks the link at build time.

Content is a Tera template too, so a literal `{{` or `{%` in a page goes
inside `{% raw %}…{% endraw %}`.

## Versions

While the island is a rolling release, there is one version, `rolling`. To
freeze the docs of a release:

```bash
scripts/freeze-version.sh 0.1.0
```

It copies `content/docs/rolling/` to `content/docs/0.1.0/`, points the copy's
links at itself, and lists `0.1.0` in `versions.toml` as the newest release.
The navigation then opens the docs on it, the rolling docs stay one choice
away in the switcher, and an older release says which one is the newest. The
switcher keeps you on the same page when the other version has it, and takes
you to that version's first page when it does not.

## Screenshots

The gallery holds concept images (`01-floating.svg` …) until there are
screenshots. Replace them with PNG or WebP files, about 1600×900, and update the
captions in `content/gallery/_index.md`; the file names set the order.
