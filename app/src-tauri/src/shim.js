/*!
 * ToolsBoXApp - shim injected into every page loaded by the WebView.
 *
 * Responsibilities:
 *   1. Downloads: pages generate blobs (`<a download href="blob:...">`)
 *      that the WebView cannot save -> native save dialog.
 *   2. External links: opened in the system browser, never inside the app.
 *   3. Clipboard: fallback through `execCommand('copy')` when the modern
 *      API fails.
 *
 * Updates are handled outside the app through the `tbxa` terminal command
 * (`tbxa update` / `tbxa uninstall`).
 *
 * The shim is re-injected on every navigation: it is idempotent per document.
 */
(function () {
  'use strict';

  if (window.__TBX_SHIM_ACTIVE__) {
    return;
  }
  window.__TBX_SHIM_ACTIVE__ = true;

  /* ------------------------------------------------------------------ *
   *  Access to the Tauri IPC (window.__TAURI__ or __TAURI_INTERNALS__)
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
      return Promise.reject(new Error('Tauri IPC unavailable'));
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
      console.log.apply(console, ['[ToolsBoXApp]'].concat([].slice.call(arguments)));
    }
  }

  /* ------------------------------------------------------------------ *
   *  Styles (injected once)
   * ------------------------------------------------------------------ */
  var CSS = [
    '[data-tbx-ui]{all:initial;box-sizing:border-box;font-family:"Segoe UI",system-ui,sans-serif;}',
    '[data-tbx-toast]{position:fixed;right:16px;bottom:16px;z-index:2147483646;display:flex;',
    '  align-items:center;gap:12px;max-width:420px;padding:10px 14px;border-radius:10px;',
    '  background:#111827f2;color:#f9fafb;font-size:13px;line-height:1.4;',
    '  box-shadow:0 8px 24px #0006;backdrop-filter:blur(4px);}',
    '[data-tbx-toast] button{all:unset;cursor:pointer;padding:4px 10px;border-radius:6px;',
    '  background:#2563eb;color:#fff;font-size:12px;font-weight:600;white-space:nowrap;}',
    '[data-tbx-toast] button:hover{background:#1d4ed8;}'
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

  /* ------------------------------------------------------------------ *
   *  1. Downloads (blob: and data:)
   * ------------------------------------------------------------------ */
  function fileNameFromUrl(url, fallback) {
    var name = fallback || 'file';
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
          toast('Saved: ' + saved, 'Open folder', function () {
            invoke('reveal_file', { path: saved }).catch(function (error) {
              log('reveal_file failed:', error);
            });
          });
        }
        return saved;
      })
      .catch(function (error) {
        log('save_download failed:', error);
        toast('Could not save: ' + error);
      });
  }

  /* ------------------------------------------------------------------ *
   *  2 & 3. Clicks: downloads and external links
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

    // 1. Download requested by the page: <a download href="...">
    var wantsDownload = anchor.hasAttribute('download')
      || anchor.hasAttribute('data-tbx-download');
    var isRemote = /^(https?:|mailto:)/i.test(url);

    if (wantsDownload && !isRemote) {
      event.preventDefault();
      var name = anchor.getAttribute('download')
        || anchor.getAttribute('data-tbx-download')
        || 'file';
      if (!/\.[a-z0-9]{1,5}$/i.test(name)) {
        name = fileNameFromUrl(url, name);
      }
      saveFromUrl(url, name);
      return;
    }

    // 2. External link or new-tab opener -> system browser
    var target = anchor.getAttribute('target');
    if (!isRemote && target !== '_blank') {
      return;
    }

    event.preventDefault();
    invoke('open_external', { url: url }).catch(function (error) {
      log('open_external failed:', error);
    });
  }

  /* ------------------------------------------------------------------ *
   *  4. Clipboard (fallback)
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
      catch (error) { /* ignored */ }

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
        reject(new Error('copy refused by the system'));
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
        saveFromUrl(url, fileNameFromUrl(url, 'file'));
        return null;
      }
      if (typeof url === 'string' && /^(https?:|mailto:)/i.test(url)) {
        invoke('open_external', { url: url }).catch(function (error) {
          log('open_external failed:', error);
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
   *  Bootstrap
   * ------------------------------------------------------------------ */
  function boot() {
    injectStyles();
    patchClipboard();
    patchWindowOpen();
    document.addEventListener('click', onClick, true);
    log('shim active on', window.location.pathname);
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', boot, { once: true });
  }
  else {
    boot();
  }
})();
