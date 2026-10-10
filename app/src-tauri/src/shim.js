/*!
 * ToolBoXApp — shim injecté dans chaque page du WebView.
 *
 * Responsabilités :
 *   1. Téléchargements : les pages génèrent des blobs (`<a download href="blob:…">`)
 *      que le WebView ne sait pas enregistrer -> boîte de dialogue native.
 *   2. Liens externes : ouverts dans le navigateur système, jamais dans l'app.
 *   3. Presse-papier : repli via `execCommand('copy')` si l'API moderne échoue.
 *   4. Mises à jour : interrogation de la dernière release GitHub au démarrage
 *      (au maximum toutes les 6 h) + bandeau « Mettre à jour ».
 *
 * Le shim est ré-injecté à chaque navigation : il est idempotent par document.
 */
(function () {
  'use strict';

  if (window.__TBX_SHIM_ACTIVE__) {
    return;
  }
  window.__TBX_SHIM_ACTIVE__ = true;

  /* ------------------------------------------------------------------ *
   *  Accès à l'IPC Tauri (window.__TAURI__ ou __TAURI_INTERNALS__)
   * ------------------------------------------------------------------ */
  function ipc() {
    var global = window.__TAURI__;
    if (global && global.core && typeof global.core.invoke === 'function') {
      return global.core;
    }
    var internals = window.__TAURI_INTERNALS__;
    if (internals && typeof internals.invoke === 'function') {
      return internals;
    }
    return null;
  }

  function invoke(command, payload, options) {
    var api = ipc();
    if (!api) {
      return Promise.reject(new Error('IPC Tauri indisponible'));
    }
    try {
      return Promise.resolve(api.invoke(command, payload, options));
    }
    catch (error) {
      return Promise.reject(error);
    }
  }

  function log() {
    if (window.console && console.log) {
      console.log.apply(console, ['[ToolBoXApp]'].concat([].slice.call(arguments)));
    }
  }

  /* ------------------------------------------------------------------ *
   *  Styles (injectés une fois)
   * ------------------------------------------------------------------ */
  var CSS = [
    '[data-tbx-ui]{all:initial;box-sizing:border-box;font-family:"Segoe UI",system-ui,sans-serif;}',
    '[data-tbx-toast]{position:fixed;right:16px;bottom:16px;z-index:2147483646;display:flex;',
    '  align-items:center;gap:12px;max-width:420px;padding:10px 14px;border-radius:10px;',
    '  background:#111827f2;color:#f9fafb;font-size:13px;line-height:1.4;',
    '  box-shadow:0 8px 24px #0006;backdrop-filter:blur(4px);}',
    '[data-tbx-toast] button{all:unset;cursor:pointer;padding:4px 10px;border-radius:6px;',
    '  background:#2563eb;color:#fff;font-size:12px;font-weight:600;white-space:nowrap;}',
    '[data-tbx-toast] button:hover{background:#1d4ed8;}',
    '[data-tbx-update]{position:fixed;right:16px;bottom:16px;z-index:2147483647;',
    '  display:flex;flex-direction:column;gap:10px;max-width:340px;padding:14px 16px;',
    '  border-radius:12px;background:#0f172af5;color:#e2e8f0;font-size:13px;line-height:1.45;',
    '  box-shadow:0 12px 32px #00000073;border:1px solid #2563eb66;}',
    '[data-tbx-update] strong{color:#fff;font-size:14px;}',
    '[data-tbx-update] .tbx-actions{display:flex;gap:8px;justify-content:flex-end;}',
    '[data-tbx-update] button{all:unset;cursor:pointer;padding:6px 12px;border-radius:7px;',
    '  font-size:12px;font-weight:600;}',
    '[data-tbx-update] .tbx-primary{background:#2563eb;color:#fff;}',
    '[data-tbx-update] .tbx-primary:hover{background:#1d4ed8;}',
    '[data-tbx-update] .tbx-ghost{color:#94a3b8;}',
    '[data-tbx-update] .tbx-ghost:hover{color:#e2e8f0;}'
  ].join('\n');

  function injectStyles() {
    if (document.getElementById('tbx-shim-styles')) {
      return;
    }
    var style = document.createElement('style');
    style.id = 'tbx-shim-styles';
    style.textContent = CSS;
    (document.head || document.documentElement).appendChild(style);
  }

  /* ------------------------------------------------------------------ *
   *  Notifications
   * ------------------------------------------------------------------ */
  function toast(message, actionLabel, action, ttl) {
    if (!document.body) {
      return;
    }
    var box = document.createElement('div');
    box.setAttribute('data-tbx-ui', '');
    box.setAttribute('data-tbx-toast', '');

    var label = document.createElement('span');
    label.textContent = message;
    box.appendChild(label);

    if (actionLabel && typeof action === 'function') {
      var button = document.createElement('button');
      button.type = 'button';
      button.textContent = actionLabel;
      button.addEventListener('click', function () {
        close();
        action();
      });
      box.appendChild(button);
    }

    var timer = null;
    function close() {
      if (timer) {
        window.clearTimeout(timer);
      }
      if (box.parentNode) {
        box.parentNode.removeChild(box);
      }
    }

    document.body.appendChild(box);
    timer = window.setTimeout(close, typeof ttl === 'number' ? ttl : 7000);
  }

  function showUpdateBanner(info) {
    if (!document.body || document.querySelector('[data-tbx-update]')) {
      return;
    }

    var box = document.createElement('div');
    box.setAttribute('data-tbx-ui', '');
    box.setAttribute('data-tbx-update', '');

    var title = document.createElement('strong');
    title.textContent = 'Mise à jour disponible';
    box.appendChild(title);

    var line = document.createElement('span');
    line.textContent = 'Version ' + (info.latestVersion || '?')
      + ' (actuelle : ' + (info.currentVersion || '?') + ')';
    box.appendChild(line);

    var actions = document.createElement('div');
    actions.className = 'tbx-actions';

    var later = document.createElement('button');
    later.type = 'button';
    later.className = 'tbx-ghost';
    later.textContent = 'Plus tard';
    later.addEventListener('click', function () {
      try {
        window.localStorage.setItem(STORAGE_SKIPPED, new Date().toDateString());
      }
      catch (error) { /* stockage indisponible */ }
      if (box.parentNode) {
        box.parentNode.removeChild(box);
      }
    });

    var update = document.createElement('button');
    update.type = 'button';
    update.className = 'tbx-primary';
    update.textContent = 'Mettre à jour';
    update.addEventListener('click', function () {
      update.disabled = true;
      update.textContent = 'Ouverture…';
      var target = info.downloadUrl || info.releaseUrl;
      invoke('open_external', { url: target })
        .catch(function (error) {
          log('open_external a échoué :', error);
          toast('Ouverture impossible : ' + error);
        })
        .then(function () {
          if (box.parentNode) {
            box.parentNode.removeChild(box);
          }
        });
    });

    actions.appendChild(later);
    actions.appendChild(update);
    box.appendChild(actions);
    document.body.appendChild(box);

    log('mise à jour disponible :', info.latestVersion, '->', info.downloadUrl || info.releaseUrl);
  }

  /* ------------------------------------------------------------------ *
   *  1. Téléchargements (blob: et data:)
   * ------------------------------------------------------------------ */
  function fileNameFromUrl(url, fallback) {
    var name = fallback || 'fichier';
    if (!url || url.indexOf('data:') !== 0) {
      return name;
    }
    var match = /filename\*?=([^;]+)/i.exec(url);
    if (match) {
      try {
        return decodeURIComponent(match[1].replace(/^UTF-8''/i, '').replace(/"/g, ''));
      }
      catch (error) {
        return name;
      }
    }
    var type = /^data:([^;,]+)/.exec(url);
    if (type && type[1].indexOf('/') !== -1) {
      var extension = type[1].split('/')[1].split('+')[0];
      if (extension && extension.length < 6) {
        return name + '.' + extension;
      }
    }
    return name;
  }

  function saveFromUrl(url, name) {
    return fetch(url)
      .then(function (response) {
        if (!response.ok) {
          throw new Error('HTTP ' + response.status);
        }
        return response.blob();
      })
      .then(function (blob) {
        return blob.arrayBuffer();
      })
      .then(function (buffer) {
        return invoke('save_download', new Uint8Array(buffer), {
          headers: {
            'x-tbx-filename': encodeURIComponent(name),
            'x-tbx-mime': 'application/octet-stream'
          }
        });
      })
      .then(function (saved) {
        if (typeof saved === 'string' && saved) {
          toast('Enregistré : ' + saved, 'Ouvrir le dossier', function () {
            invoke('reveal_file', { path: saved }).catch(function (error) {
              log('reveal_file a échoué :', error);
            });
          });
        }
        return saved;
      })
      .catch(function (error) {
        log('save_download a échoué :', error);
        toast('Enregistrement impossible : ' + error);
      });
  }

  /* ------------------------------------------------------------------ *
   *  2 & 3. Clics : téléchargements et liens externes
   * ------------------------------------------------------------------ */
  function closestAnchor(node) {
    while (node && node !== document.body) {
      if (node.tagName === 'A') {
        return node;
      }
      node = node.parentNode;
    }
    return null;
  }

  function onClick(event) {
    var node = event.target;
    if (!node || typeof node.getAttribute !== 'function') {
      return;
    }

    var anchor = node.closest ? node.closest('a[href]') : closestAnchor(node);
    if (!anchor) {
      return;
    }

    var url = anchor.getAttribute('href') || '';
    if (!url) {
      return;
    }

    // 1. Téléchargement demandé par la page : <a download href="…">
    var wantsDownload = anchor.hasAttribute('download')
      || anchor.hasAttribute('data-tbx-download');
    var isRemote = /^(https?:|mailto:)/i.test(url);

    if (wantsDownload && !isRemote) {
      event.preventDefault();
      var name = anchor.getAttribute('download')
        || anchor.getAttribute('data-tbx-download')
        || 'fichier';
      if (!/\.[a-z0-9]{1,5}$/i.test(name)) {
        name = fileNameFromUrl(url, name);
      }
      saveFromUrl(url, name);
      return;
    }

    // 2. Lien externe ou ouverture dans un nouvel onglet -> navigateur système
    var target = anchor.getAttribute('target');
    if (!isRemote && target !== '_blank') {
      return;
    }

    event.preventDefault();
    invoke('open_external', { url: url }).catch(function (error) {
      log('open_external a échoué :', error);
    });
  }

  /* ------------------------------------------------------------------ *
   *  4. Presse-papier (repli)
   * ------------------------------------------------------------------ */
  function legacyCopy(text) {
    return new Promise(function (resolve, reject) {
      var area = document.createElement('textarea');
      area.value = text === null || text === undefined ? '' : String(text);
      area.setAttribute('readonly', '');
      area.style.position = 'fixed';
      area.style.top = '0';
      area.style.left = '0';
      area.style.opacity = '0';
      (document.body || document.documentElement).appendChild(area);
      area.select();
      try {
        area.setSelectionRange(0, area.value.length);
      }
      catch (error) { /* ignoré */ }

      var ok = false;
      try {
        ok = document.execCommand('copy');
      }
      catch (error) {
        ok = false;
      }
      if (area.parentNode) {
        area.parentNode.removeChild(area);
      }
      if (ok) {
        resolve();
      }
      else {
        reject(new Error('copie refusée par le système'));
      }
    });
  }

  function patchClipboard() {
    var clipboard = navigator.clipboard;
    if (!clipboard) {
      return;
    }
    if (typeof clipboard.writeText === 'function') {
      var nativeWriteText = clipboard.writeText.bind(clipboard);
      clipboard.writeText = function (text) {
        return Promise.resolve()
          .then(function () {
            return nativeWriteText(text);
          })
          .catch(function () {
            return legacyCopy(text);
          });
      };
    }
  }

  /* ------------------------------------------------------------------ *
   *  5. window.open
   * ------------------------------------------------------------------ */
  function patchWindowOpen() {
    var nativeOpen = window.open;
    window.open = function (url, name, features) {
      if (typeof url === 'string' && (url.indexOf('blob:') === 0 || url.indexOf('data:') === 0)) {
        saveFromUrl(url, fileNameFromUrl(url, 'fichier'));
        return null;
      }
      if (typeof url === 'string' && /^(https?:|mailto:)/i.test(url)) {
        invoke('open_external', { url: url }).catch(function (error) {
          log('open_external a échoué :', error);
        });
        return null;
      }
      if (typeof nativeOpen === 'function') {
        return nativeOpen.call(window, url, name, features);
      }
      return null;
    };
  }

  /* ------------------------------------------------------------------ *
   *  6. Vérification des mises à jour
   * ------------------------------------------------------------------ */
  var STORAGE_KEY = 'toolbox.update.lastCheck';
  var STORAGE_SKIPPED = 'toolbox.update.skipped';
  var CHECK_INTERVAL_MS = 6 * 60 * 60 * 1000;

  function checkForUpdate() {
    var now = Date.now();
    var last = 0;
    try {
      last = parseInt(window.localStorage.getItem(STORAGE_KEY) || '0', 10) || 0;
      if (window.localStorage.getItem(STORAGE_SKIPPED) === String(new Date().toDateString())) {
        return;
      }
    }
    catch (error) {
      return;
    }

    if (now - last < CHECK_INTERVAL_MS) {
      return;
    }
    try {
      window.localStorage.setItem(STORAGE_KEY, String(now));
    }
    catch (error) { /* stockage indisponible : on continue */ }

    invoke('check_update')
      .then(function (info) {
        if (!info) {
          return;
        }
        if (info.error) {
          log('check_update :', info.error);
        }
        if (info.available) {
          showUpdateBanner(info);
        }
      })
      .catch(function (error) {
        log('check_update a échoué :', error);
      });
  }

  /* ------------------------------------------------------------------ *
   *  Amorçage
   * ------------------------------------------------------------------ */
  function boot() {
    injectStyles();
    patchClipboard();
    patchWindowOpen();
    document.addEventListener('click', onClick, true);
    checkForUpdate();
    log('shim actif sur', window.location.pathname);
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', boot, { once: true });
  }
  else {
    boot();
  }
})();
