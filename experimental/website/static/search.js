// The docs' search: a dialog over the page, opened from the navbar, with "/"
// or Ctrl+K. The index Zola builds is loaded the first time it opens, and
// only the docs of the version being read (or the newest, off the docs)
// are searched.
(() => {
  const dialog = document.getElementById("search-dialog");
  const input = document.getElementById("search-input");
  const list = document.getElementById("search-results");
  const { index: indexUrl, lunr: lunrUrl, version: defaultVersion } = dialog.dataset;
  const version = location.pathname.match(/\/docs\/([^/]+)\//)?.[1] ?? defaultVersion;
  const MAX = 8;

  let index;
  let selected = 0;

  const load = () => {
    index ??= new Promise((resolve, reject) => {
      const script = document.createElement("script");
      script.src = lunrUrl;
      script.onload = resolve;
      script.onerror = reject;
      document.head.append(script);
    })
      .then(() => fetch(indexUrl))
      .then((response) => response.json())
      .then((data) => elasticlunr.Index.load(data));
    return index;
  };

  const escape = (text) =>
    text.replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]);

  // Some words round the first term found, the terms in bold.
  const teaser = (body, terms) => {
    const lower = body.toLowerCase();
    const first = terms.map((t) => lower.indexOf(t)).filter((i) => i >= 0).sort((a, b) => a - b)[0] ?? 0;
    const start = Math.max(0, first - 60);
    let text = escape((start > 0 ? "…" : "") + body.slice(start, start + 160) + "…");
    for (const term of terms) {
      if (term) text = text.replace(new RegExp(`(${term.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")})`, "gi"), "<b>$1</b>");
    }
    return text;
  };

  const select = (i) => {
    const items = [...list.children];
    if (items.length === 0) return;
    selected = (i + items.length) % items.length;
    items.forEach((item, n) => item.classList.toggle("selected", n === selected));
    items[selected].scrollIntoView({ block: "nearest" });
  };

  const search = async () => {
    const term = input.value.trim();
    if (term === "") {
      list.innerHTML = "";
      return;
    }

    const results = (await load())
      .search(term, { bool: "AND", expand: true, fields: { title: { boost: 3 }, description: { boost: 2 }, body: { boost: 1 } } })
      .filter((result) => result.ref.includes(`/docs/${version}/`))
      .slice(0, MAX);
    if (input.value.trim() !== term) return;

    const terms = term.toLowerCase().split(/\s+/);
    list.innerHTML = results.length
      ? results
          .map(
            ({ ref, doc }) =>
              `<li><a href="${ref}"><span class="search-title">${escape(doc.title)}</span>` +
              `<span class="search-teaser">${teaser(doc.body, terms)}</span></a></li>`,
          )
          .join("")
      : `<li class="search-empty">No results for “${escape(term)}”</li>`;
    select(0);
  };

  const open = () => {
    if (!dialog.open) dialog.showModal();
    input.select();
    load();
  };

  document.querySelectorAll(".search-button").forEach((button) => button.addEventListener("click", open));
  document.addEventListener("keydown", (event) => {
    const typing = /^(INPUT|TEXTAREA|SELECT)$/.test(document.activeElement?.tagName);
    if ((event.key === "/" && !typing) || (event.key === "k" && (event.ctrlKey || event.metaKey))) {
      event.preventDefault();
      open();
    }
  });

  let pending;
  input.addEventListener("input", () => {
    clearTimeout(pending);
    pending = setTimeout(search, 120);
  });
  input.addEventListener("keydown", (event) => {
    if (event.key === "ArrowDown") select(selected + 1);
    else if (event.key === "ArrowUp") select(selected - 1);
    else if (event.key === "Enter") list.children[selected]?.querySelector("a")?.click();
    else return;
    event.preventDefault();
  });

  // A click outside the box closes it.
  dialog.addEventListener("click", (event) => {
    if (event.target === dialog) dialog.close();
  });
})();
