
(function () {
  "use strict";

  const THEME_KEY = "nvide-arch-theme";
  const DETAILS_KEY = "nvide-arch-details";
  const root = document.documentElement;
  const sidebar = document.getElementById("sidebar");
  const tocEl = document.getElementById("toc");
  const searchInput = document.getElementById("search-input");
  const searchCount = document.getElementById("search-count");
  const backToTop = document.getElementById("back-to-top");
  const themeToggle = document.getElementById("theme-toggle");
  const sidebarToggle = document.getElementById("sidebar-toggle");


  function systemTheme() {
    return window.matchMedia("(prefers-color-scheme: dark)").matches
      ? "dark"
      : "light";
  }
  function applyTheme(theme) {
    if (theme === "system") {
      root.removeAttribute("data-theme");
      themeToggle.textContent = "Theme";
    } else {
      root.setAttribute("data-theme", theme);
      themeToggle.textContent = theme === "dark" ? "Light" : "Dark";
    }
  }
  function loadTheme() {
    const saved = localStorage.getItem(THEME_KEY);
    if (saved === "light" || saved === "dark") applyTheme(saved);
    else applyTheme("system");
  }
  themeToggle.addEventListener("click", function () {
    const current = root.getAttribute("data-theme");
    let next;
    if (!current) next = systemTheme() === "dark" ? "light" : "dark";
    else next = current === "dark" ? "light" : "dark";
    localStorage.setItem(THEME_KEY, next);
    applyTheme(next);
  });
  loadTheme();


  const main = document.querySelector("main");

  const headings = main.querySelectorAll("section h2, section h3");
  const tocItems = [];
  const usedIds = new Set();

  function slugify(text, idx) {
    const base =
      (text || "section")
        .toLowerCase()
        .replace(/[^\w\s-]/g, "")
        .trim()
        .replace(/\s+/g, "-")
        .slice(0, 60) || "section";
    let id = base;
    let n = 2;
    while (usedIds.has(id) || document.getElementById(id)) {
      id = base + "-" + n;
      n += 1;
    }
    return id;
  }

  headings.forEach(function (h, idx) {
    if (h.closest(".cover")) return;

    if (!h.id) {
      const host = h.closest("article[id], section[id]");
      if (host && host.id && !usedIds.has(host.id)) {

        const claimed = document.getElementById(host.id);
        if (!claimed || claimed === host) {
          h.id = host.id;
          host.removeAttribute("id");
        }
      }
    }
    if (!h.id) {
      h.id = slugify(h.textContent, idx);
    }
    usedIds.add(h.id);
    const li = document.createElement("li");
    li.className = "toc-" + h.tagName.toLowerCase();
    const a = document.createElement("a");
    a.href = "#" + h.id;
    a.textContent = h.textContent.trim();
    li.appendChild(a);
    tocEl.appendChild(li);
    tocItems.push({ id: h.id, el: h, link: a });
  });


  let highlightLockId = null;
  let highlightLockTimer = null;
  let lastActiveId = null;


  function getAnchorOffset() {
    const pad = getComputedStyle(document.documentElement).scrollPaddingTop;
    const parsed = parseFloat(pad);
    if (!isNaN(parsed) && parsed > 0) return parsed;
    const header = document.querySelector(".doc-header");
    return (header ? header.offsetHeight : 52) + 16;
  }

  function setActiveTocItem(current) {
    if (!current) return;
    const same = lastActiveId === current.id;
    lastActiveId = current.id;
    tocItems.forEach(function (item) {
      item.link.classList.toggle("active", item === current);
    });
    if (same) return;
    const link = current.link;
    if (!sidebar || !link) return;
    const linkRect = link.getBoundingClientRect();
    const sideRect = sidebar.getBoundingClientRect();
    if (
      linkRect.top < sideRect.top + 8 ||
      linkRect.bottom > sideRect.bottom - 8
    ) {
      link.scrollIntoView({ block: "nearest", inline: "nearest" });
    }
  }

  function sectionBounds(i) {
    const start = tocItems[i].el.getBoundingClientRect().top;
    var end;
    if (i + 1 < tocItems.length) {
      end = tocItems[i + 1].el.getBoundingClientRect().top;
    } else {
      const mainEl = document.querySelector("main");
      end = mainEl
        ? mainEl.getBoundingClientRect().bottom
        : document.documentElement.scrollHeight - window.scrollY;
    }
    return { start: start, end: end };
  }

  function updateActiveSection() {
    if (!tocItems.length) return;

    if (highlightLockId) {
      const locked = tocItems.find(function (item) {
        return item.id === highlightLockId;
      });
      if (locked) {
        setActiveTocItem(locked);
        return;
      }
    }

    const anchor = getAnchorOffset();
    const vh = window.innerHeight;

    const bandTop = anchor;
    const bandBottom = vh;
    const bandH = Math.max(1, bandBottom - bandTop);

    var bestIdx = 0;
    var bestRatio = -1;
    var bestOwnsAnchor = false;

    for (var i = 0; i < tocItems.length; i++) {
      const b = sectionBounds(i);

      const top = Math.max(b.start, bandTop);
      const bottom = Math.min(b.end, bandBottom);
      const overlap = Math.max(0, bottom - top);
      const ratio = overlap / bandH;

      const ownsAnchor = b.start <= anchor + 0.5 && b.end > anchor + 0.5;


      var take = false;
      if (ratio > bestRatio + 0.02) take = true;
      else if (Math.abs(ratio - bestRatio) <= 0.02) {
        if (ownsAnchor && !bestOwnsAnchor) take = true;
        else if (ownsAnchor === bestOwnsAnchor && ratio >= bestRatio && i >= bestIdx)
          take = true;
      }

      if (take && (ratio > 0 || ownsAnchor)) {
        bestRatio = ratio;
        bestIdx = i;
        bestOwnsAnchor = ownsAnchor;
      }
    }


    if (bestRatio <= 0 && !bestOwnsAnchor) {
      bestIdx = 0;
      for (var j = 0; j < tocItems.length; j++) {
        if (tocItems[j].el.getBoundingClientRect().top <= anchor + 0.5)
          bestIdx = j;
      }
    }


    const doc = document.documentElement;
    const atBottom =
      window.innerHeight + window.scrollY >= doc.scrollHeight - 4;
    if (atBottom && tocItems.length) {
      const last = tocItems.length - 1;
      const lb = sectionBounds(last);
      if (lb.start < vh) bestIdx = last;
    }

    setActiveTocItem(tocItems[bestIdx]);
  }

  function lockHighlight(id, ms) {
    highlightLockId = id;
    clearTimeout(highlightLockTimer);
    highlightLockTimer = setTimeout(function () {
      highlightLockId = null;
      updateActiveSection();
    }, ms || 900);
  }


  tocEl.addEventListener("click", function (e) {
    const a = e.target.closest("a");
    if (!a) return;
    const id = (a.getAttribute("href") || "").replace(/^#/, "");
    if (!id) return;
    const target = tocItems.find(function (item) {
      return item.id === id;
    });
    if (target) {
      setActiveTocItem(target);
      lockHighlight(id, 1000);
    }
    if (sidebar) sidebar.classList.remove("open");
    if (sidebarToggle) sidebarToggle.setAttribute("aria-expanded", "false");
  });

  window.addEventListener("scroll", updateActiveSection, { passive: true });
  window.addEventListener("resize", updateActiveSection, { passive: true });
  if ("onscrollend" in window) {
    window.addEventListener("scrollend", function () {
      highlightLockId = null;
      updateActiveSection();
    });
  }
  window.addEventListener("hashchange", function () {
    const id = (location.hash || "").replace(/^#/, "");
    if (!id) {
      updateActiveSection();
      return;
    }
    const target = tocItems.find(function (item) {
      return item.id === id;
    });
    if (target) {
      setActiveTocItem(target);
      lockHighlight(id, 1000);
    } else {
      updateActiveSection();
    }
  });
  (function initHighlight() {
    const id = (location.hash || "").replace(/^#/, "");
    const target =
      id &&
      tocItems.find(function (item) {
        return item.id === id;
      });
    if (target) {
      setActiveTocItem(target);
      lockHighlight(id, 500);
    } else {
      updateActiveSection();
    }
  })();



  function updateBackToTop() {
    backToTop.classList.toggle("visible", window.scrollY > 400);
  }
  window.addEventListener("scroll", updateBackToTop, { passive: true });
  backToTop.addEventListener("click", function () {
    window.scrollTo({ top: 0, behavior: "smooth" });
  });


  if (sidebarToggle) {
    sidebarToggle.setAttribute("aria-controls", sidebar.id);
    sidebarToggle.setAttribute("aria-expanded", "false");
    sidebarToggle.addEventListener("click", function () {
      sidebar.classList.toggle("open");
      sidebarToggle.setAttribute(
        "aria-expanded",
        String(sidebar.classList.contains("open")),
      );
    });
    document.addEventListener("click", function (e) {
      if (!sidebar.classList.contains("open")) return;
      if (sidebar.contains(e.target) || sidebarToggle.contains(e.target))
        return;
      sidebar.classList.remove("open");
      sidebarToggle.setAttribute("aria-expanded", "false");
    });
  }


  function allDetails() {
    return Array.prototype.slice.call(
      document.querySelectorAll("details"),
    );
  }
  function saveDetailsState() {
    const state = {};
    allDetails().forEach(function (d, i) {
      const key = d.querySelector("summary")
        ? d.querySelector("summary").textContent.trim()
        : String(i);
      state[key] = d.open;
    });
    try {
      localStorage.setItem(DETAILS_KEY, JSON.stringify(state));
    } catch (e) {}
  }
  function loadDetailsState() {
    try {
      const raw = localStorage.getItem(DETAILS_KEY);
      if (!raw) return;
      const state = JSON.parse(raw);
      allDetails().forEach(function (d, i) {
        const key = d.querySelector("summary")
          ? d.querySelector("summary").textContent.trim()
          : String(i);
        if (Object.prototype.hasOwnProperty.call(state, key))
          d.open = !!state[key];
      });
    } catch (e) {}
  }
  allDetails().forEach(function (d) {
    d.addEventListener("toggle", saveDetailsState);
  });
  loadDetailsState();

  const expandButton = document.getElementById("btn-expand");
  const collapseButton = document.getElementById("btn-collapse");
  if (expandButton)
    expandButton.addEventListener("click", function () {
      allDetails().forEach(function (d) {
        d.open = true;
      });
      saveDetailsState();
    });
  if (collapseButton)
    collapseButton.addEventListener("click", function () {
      allDetails().forEach(function (d) {
        d.open = false;
      });
      saveDetailsState();
    });


  document.querySelectorAll("pre").forEach(function (pre) {
    if (pre.closest(".diagram")) return;
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "copy-btn";
    btn.textContent = "Copy";
    btn.addEventListener("click", function () {
      const code = pre.querySelector("code");
      const text = code ? code.innerText : pre.innerText;
      navigator.clipboard
        .writeText(text)
        .then(function () {
          btn.textContent = "Copied";
          setTimeout(function () {
            btn.textContent = "Copy";
          }, 1500);
        })
        .catch(function () {
          btn.textContent = "Fail";
          setTimeout(function () {
            btn.textContent = "Copy";
          }, 1500);
        });
    });
    pre.appendChild(btn);
  });


  let hitNodes = [];
  let currentHit = -1;

  function clearSearchHighlights() {
    document.querySelectorAll("mark.search-hit").forEach(function (m) {
      const parent = m.parentNode;
      parent.replaceChild(document.createTextNode(m.textContent), m);
      parent.normalize();
    });
    hitNodes = [];
    currentHit = -1;
    searchCount.textContent = "";
  }

  function walkTextNodes(root, cb) {
    const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT, {
      acceptNode: function (node) {
        if (!node.nodeValue || !node.nodeValue.trim())
          return NodeFilter.FILTER_REJECT;
        const p = node.parentElement;
        if (!p) return NodeFilter.FILTER_REJECT;
        const tag = p.tagName;
        if (tag === "SCRIPT" || tag === "STYLE" || tag === "NOSCRIPT")
          return NodeFilter.FILTER_REJECT;
        if (p.closest(".doc-header, .sidebar, .copy-btn"))
          return NodeFilter.FILTER_REJECT;
        return NodeFilter.FILTER_ACCEPT;
      },
    });
    const nodes = [];
    while (walker.nextNode()) nodes.push(walker.currentNode);
    nodes.forEach(cb);
  }

  function performSearch(query) {
    clearSearchHighlights();
    if (!query || query.length < 2) return;
    const q = query.toLowerCase();
    const contentRoot = document.querySelector("main");
    walkTextNodes(contentRoot, function (textNode) {
      const text = textNode.nodeValue;
      const lower = text.toLowerCase();
      let idx = lower.indexOf(q);
      if (idx === -1) return;
      const frag = document.createDocumentFragment();
      let last = 0;
      while (idx !== -1) {
        if (idx > last)
          frag.appendChild(
            document.createTextNode(text.slice(last, idx)),
          );
        const mark = document.createElement("mark");
        mark.className = "search-hit";
        mark.textContent = text.slice(idx, idx + query.length);
        frag.appendChild(mark);
        hitNodes.push(mark);
        last = idx + query.length;
        idx = lower.indexOf(q, last);
      }
      if (last < text.length)
        frag.appendChild(document.createTextNode(text.slice(last)));
      textNode.parentNode.replaceChild(frag, textNode);
    });
    searchCount.textContent = hitNodes.length
      ? hitNodes.length + " hits"
      : "0 hits";
    if (hitNodes.length) {
      currentHit = 0;
      hitNodes[0].scrollIntoView({ behavior: "smooth", block: "center" });
    }
  }

  let searchTimer;
  searchInput.addEventListener("input", function () {
    clearTimeout(searchTimer);
    searchTimer = setTimeout(function () {
      performSearch(searchInput.value.trim());
    }, 200);
  });
  searchInput.addEventListener("keydown", function (e) {
    if (e.key === "Enter" && hitNodes.length) {
      e.preventDefault();
      currentHit =
        (currentHit + (e.shiftKey ? -1 : 1) + hitNodes.length) %
        hitNodes.length;
      hitNodes[currentHit].scrollIntoView({
        behavior: "smooth",
        block: "center",
      });
      searchCount.textContent = currentHit + 1 + " / " + hitNodes.length;
    }
    if (e.key === "Escape") {
      searchInput.value = "";
      clearSearchHighlights();
      searchInput.blur();
    }
  });
})();
