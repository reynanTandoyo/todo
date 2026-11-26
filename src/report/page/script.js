/* The script of the v1.0 page (T-203; R-REPORT-6; section 5.23). It is copied into the page inline,
   as the only script, and it changes only what the page shows: the mode, the colour set, the view
   filter, the archive's place in All work and the open state of the milestones. It names no address
   and calls no network function.
   Every use of localStorage is wrapped, so a browser that refuses storage still shows the page. */
(function () {
  var page = document.querySelector('.page');
  page.classList.add('js');

  var THEME_KEY = 'todo-report-theme';
  var CB_KEY = 'todo-report-cb';
  var tree = document.getElementById('tree');
  var archButton = document.getElementById('arch');
  var archTree = document.getElementById('arch-tree');
  var earlierFold = document.getElementById('earlier');
  var overallNow = document.getElementById('overall-now');
  var overallAll = document.getElementById('overall-all');
  var ARCH_KEY = 'todo-report-archive';
  var archiveIncluded = false;
  var currentFilter = 'all';
  var themeButton = document.getElementById('theme');
  var cbButton = document.getElementById('cb');
  var filterButtons = document.querySelectorAll('.controls button[data-f]');
  var emptyView = document.getElementById('empty-view');
  var EMPTY_TEXT = { left: 'Nothing still to do.', done: 'Nothing done yet.', drop: 'Nothing cancelled.' };
  var systemDark = null;
  try {
    systemDark = window.matchMedia('(prefers-color-scheme: dark)');
  } catch (e) {
    systemDark = null;
  }

  function load(key) {
    try {
      return window.localStorage.getItem(key);
    } catch (e) {
      return null;
    }
  }

  function save(key, value) {
    try {
      window.localStorage.setItem(key, value);
    } catch (e) {
      /* The choice still works for this view; it is only not remembered. */
    }
  }

  /* Light or dark mode. Until the button is used the page has no data-theme, so the stylesheet
     follows the system setting; the button names the mode it switches to. */
  function isDark() {
    var chosen = page.getAttribute('data-theme');
    if (chosen) {
      return chosen === 'dark';
    }
    return !!(systemDark && systemDark.matches);
  }

  function paintTheme() {
    themeButton.textContent = isDark() ? 'Light mode' : 'Dark mode';
    /* The stylesheet colours the page itself; the space around it takes the same background. */
    document.body.style.background = window.getComputedStyle(page).backgroundColor;
  }

  function setTheme(mode) {
    page.setAttribute('data-theme', mode);
    save(THEME_KEY, mode);
    paintTheme();
  }

  var savedTheme = load(THEME_KEY);
  if (savedTheme === 'dark' || savedTheme === 'light') {
    page.setAttribute('data-theme', savedTheme);
  }
  themeButton.addEventListener('click', function () {
    setTheme(isDark() ? 'light' : 'dark');
  });
  if (systemDark && systemDark.addEventListener) {
    systemDark.addEventListener('change', paintTheme);
  }
  paintTheme();

  /* Colour-blind colours: one set of state colours, switched on by data-cb. */
  function setColourBlind(on) {
    if (on) {
      page.setAttribute('data-cb', 'on');
    } else {
      page.removeAttribute('data-cb');
    }
    cbButton.setAttribute('aria-pressed', on ? 'true' : 'false');
  }

  setColourBlind(load(CB_KEY) === 'on');
  cbButton.addEventListener('click', function () {
    var on = page.getAttribute('data-cb') !== 'on';
    setColourBlind(on);
    save(CB_KEY, on ? 'on' : 'off');
  });

  /* The view filter. Entries that do not match are hidden, and so is every milestone left without a
     visible entry; milestones that keep one are opened. Everything shows the whole tree again. */
  function matches(item, filter) {
    if (filter === 'all') {
      return true;
    }
    if (filter === 'done') {
      return item.classList.contains('done');
    }
    if (filter === 'drop') {
      return item.classList.contains('drop');
    }
    return (
      item.classList.contains('todo') ||
      item.classList.contains('prog') ||
      item.classList.contains('wait')
    );
  }

  /* The entries of All work: the tree of TODO.md, and the archive's tree too while Include archive is
     pressed. The view filter, Expand all and Collapse all all work on this list. */
  function workItems(selector) {
    var items = Array.prototype.slice.call(tree.querySelectorAll(selector));
    if (archiveIncluded && archTree) {
      items = items.concat(Array.prototype.slice.call(archTree.querySelectorAll(selector)));
    }
    return items;
  }

  function applyFilter(filter) {
    currentFilter = filter;
    workItems('li.item').forEach(function (item) {
      item.classList.toggle('hide', !matches(item, filter));
    });
    /* Inner milestones first, so each outer one sees which of its entries are visible. */
    var milestones = workItems('details.ms');
    for (var i = milestones.length - 1; i >= 0; i--) {
      var milestone = milestones[i];
      var visible = !!milestone.querySelector('li.item:not(.hide)');
      milestone.classList.toggle('hide', filter !== 'all' && !visible);
      if (filter !== 'all' && visible) {
        milestone.open = true;
      }
    }
    filterButtons.forEach(function (button) {
      button.setAttribute(
        'aria-pressed',
        button.getAttribute('data-f') === filter ? 'true' : 'false'
      );
    });
    /* A view that leaves no visible entry names it in words; otherwise the line stays hidden. */
    if (emptyView) {
      var none = filter !== 'all' && !workItems('li.item:not(.hide)').length;
      emptyView.textContent = none ? EMPTY_TEXT[filter] : '';
      emptyView.hidden = !none;
    }
  }

  /* Include archive: the archive's tree moves into All work below the tree of TODO.md, Earlier work
     is hidden since its tree shows there, and the overall line becomes the one that counts both files.
     Pressed again, everything goes back to where it was. */
  function setArchive(on) {
    if (!archButton || !archTree) {
      return;
    }
    archiveIncluded = on;
    if (on) {
      tree.parentNode.insertBefore(archTree, tree.nextSibling);
    } else {
      earlierFold.appendChild(archTree);
      /* Back in Earlier work the archive shows all its entries again, as before the button. */
      archTree.querySelectorAll('.hide').forEach(function (item) {
        item.classList.remove('hide');
      });
    }
    earlierFold.hidden = on;
    if (overallNow && overallAll) {
      overallNow.hidden = on;
      overallAll.hidden = !on;
    }
    archButton.setAttribute('aria-pressed', on ? 'true' : 'false');
    applyFilter(currentFilter);
  }

  filterButtons.forEach(function (button) {
    button.addEventListener('click', function () {
      applyFilter(button.getAttribute('data-f'));
    });
  });

  if (archButton) {
    setArchive(load(ARCH_KEY) === 'on');
    archButton.addEventListener('click', function () {
      var on = !archiveIncluded;
      setArchive(on);
      save(ARCH_KEY, on ? 'on' : 'off');
    });
  }

  /* Expand all and Collapse all work on All work: the tree and, while included, the archive. */
  document.getElementById('exp').addEventListener('click', function () {
    workItems('details.ms').forEach(function (milestone) {
      milestone.open = true;
    });
  });
  document.getElementById('col').addEventListener('click', function () {
    workItems('details').forEach(function (details) {
      details.open = false;
    });
  });
})();
