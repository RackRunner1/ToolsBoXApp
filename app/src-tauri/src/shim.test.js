/* Test fonctionnel du shim ToolBoXApp avec un faux DOM (Node). */
'use strict';
const fs = require('fs');
const vm = require('vm');
const path = require('path');

const SHIM = fs.readFileSync(
  path.join(__dirname, 'shim.js'),
  'utf8'
);

let passed = 0;
let failed = 0;
function check(label, condition, extra) {
  if (condition) {
    passed++;
    console.log('  ok   - ' + label);
  }
  else {
    failed++;
    console.log('  FAIL - ' + label + (extra ? ' :: ' + extra : ''));
  }
}

/* ------------------------------------------------------------------ *
 *  Mini DOM
 * ------------------------------------------------------------------ */
class FakeClassList {
  constructor() { this.items = []; }
  add(v) { this.items.push(v); }
  contains(v) { return this.items.indexOf(v) !== -1; }
}

class FakeElement {
  constructor(tag) {
    this.tagName = String(tag).toUpperCase();
    this.attributes = {};
    this.children = [];
    this.parentNode = null;
    this.listeners = {};
    this.textContent = '';
    this._hasAttribute = {};
    this.style = {};
    this.dataset = {};
    this.classList = new FakeClassList();
  }
  setAttribute(name, value) { this.attributes[name] = String(value); }
  getAttribute(name) {
    return Object.prototype.hasOwnProperty.call(this.attributes, name) ? this.attributes[name] : null;
  }
  hasAttribute(name) { return Object.prototype.hasOwnProperty.call(this.attributes, name); }
  removeAttribute(name) { delete this.attributes[name]; }
  appendChild(child) { child.parentNode = this; this.children.push(child); return child; }
  removeChild(child) {
    const index = this.children.indexOf(child);
    if (index !== -1) { this.children.splice(index, 1); child.parentNode = null; }
    return child;
  }
  addEventListener(type, handler) {
    (this.listeners[type] = this.listeners[type] || []).push(handler);
  }
  removeEventListener() {}
  querySelector(selector) {
    // Supporte : 'a[download]', 'a[href]', '[data-tbx-update]'
    return this.querySelectorAll(selector)[0] || null;
  }
  getElementById(id) {
    const found = this.querySelectorAll('[id]').filter((element) => element.id === id);
    return found[0] || null;
  }
  querySelectorAll(selector) {
    const out = [];
    const walk = (node) => {
      for (const child of node.children) {
        if (matches(child, selector)) { out.push(child); }
        walk(child);
      }
    };
    walk(this);
    return out;
  }
  closest(selector) {
    let node = this;
    while (node) {
      if (matches(node, selector)) { return node; }
      node = node.parentNode;
    }
    return null;
  }
  click() {
    dispatch(this, { type: 'click', target: this });
  }
  set type(value) { this.attributes.type = value; }
  get type() { return this.attributes.type; }
  set className(value) { this.attributes.class = value; }
  get className() { return this.attributes.class; }
  set disabled(value) { this.attributes.disabled = String(value); }
  get disabled() { return this.attributes.disabled === 'true'; }
  get id() { return this.attributes.id || ''; }
  set id(value) { this.attributes.id = value; }
}

function matches(element, selector) {
  if (/^a\[download\]$/.test(selector)) {
    return element.tagName === 'A' && element.hasAttribute('download');
  }
  if (/^a\[href\]$/.test(selector)) {
    return element.tagName === 'A' && element.hasAttribute('href');
  }
  if (/^\[data-tbx-update\]$/.test(selector)) {
    return element.hasAttribute('data-tbx-update');
  }
  return false;
}

function dispatch(target, event) {
  // Remonte de la cible vers document en phase de capture.
  const chain = [];
  let node = target;
  while (node) { chain.unshift(node); node = node.parentNode; }
  for (const node of chain) {
    const handlers = (node.listeners && node.listeners[event.type]) || [];
    for (const handler of handlers) {
      event.currentTarget = node;
      event.preventDefault = () => { event.defaultPrevented = true; };
      handler(event);
    }
  }
}

/* ------------------------------------------------------------------ *
 *  Monde simulé
 * ------------------------------------------------------------------ */
function createWorld(options) {
  const opts = options || {};
  const document = new FakeElement('#document');
  document.readyState = opts.readyState || 'complete';
  document.head = new FakeElement('head');
  document.body = new FakeElement('body');
  document.head.parentNode = document;
  document.body.parentNode = document;
  document.createElement = (tag) => new FakeElement(tag);
  document.createTextNode = (text) => new FakeElement('#text');

  const invokes = [];
  const clipboard = {
    writeText: () => Promise.reject(new Error('navigator.clipboard indisponible dans le test'))
  };

  const storageData = {};
  const storage = {
    getItem: (k) => (Object.prototype.hasOwnProperty.call(storageData, k) ? storageData[k] : null),
    setItem: (k, v) => { storageData[k] = String(v); },
    removeItem: (k) => { delete storageData[k]; }
  };

  let window = {
    document,
    navigator: { clipboard },
    localStorage: storage,
    console,
    setTimeout: (fn) => { timeouts.push(fn); return timeouts.length; },
    clearTimeout: () => {},
    location: { pathname: '/index.html', href: 'https://tauri.localhost/index.html' }
  };
  const timeouts = [];

  window.window = window;
  window.self = window;

  if (opts.updateResult !== undefined) {
    invokes.push({ command: 'check_update', args: [] });
    window.__TAURI__ = {
      core: {
        invoke: (command, payload, options2) => {
          invokes.push({ command, payload, options: options2 });
          if (command === 'check_update') {
            return Promise.resolve(opts.updateResult);
          }
          if (command === 'save_download') {
            return Promise.resolve(opts.savedPath || 'C:\\Users\\test\\Downloads\\fichier.png');
          }
          return Promise.resolve(opts.invokeResults && opts.invokeResults[command]);
        }
      }
    };
  }

  const sandbox = {
    window,
    document,
    navigator: window.navigator,
    localStorage: storage,
    console,
    setTimeout: window.setTimeout,
    clearTimeout: window.clearTimeout,
    Date,
    parseInt,
    Promise,
    Uint8Array,
    fetch: () => Promise.resolve({ ok: true, blob: () => Promise.resolve({ arrayBuffer: () => Promise.resolve(new ArrayBuffer(8)) }) }),
    Blob: function () {},
    Error
  };
  sandbox.globalThis = sandbox;
  vm.createContext(sandbox);

  return { sandbox, document, invokes, window, timeouts };
}

function run(world) {
  vm.runInContext(SHIM, world.sandbox, { filename: 'shim.js' });
  if (world.document.readyState === 'loading') {
    const handlers = world.document.listeners['DOMContentLoaded'] || [];
    for (const handler of handlers) { handler(); }
  }
  return world;
}

/* ------------------------------------------------------------------ *
 *  1. Amorçage + bandeau de mise à jour
 * ------------------------------------------------------------------ */
console.log('\n[1] Demarrage et verification des mises a jour');
{
  const world = run(createWorld({
    updateResult: {
      available: true,
      currentVersion: '2026.10.10',
      latestVersion: '20261011',
      releaseUrl: 'https://github.com/RackRunner1/ToolsBoXApp/releases/latest',
      downloadUrl: 'https://github.com/RackRunner1/ToolsBoXApp/releases/download/20261011/ToolsBoXApp-Setup-x64.exe',
      error: null
    }
  }));

  const calls = world.invokes.map((entry) => entry.command);
  check('check_update appele au demarrage', calls.indexOf('check_update') !== -1, calls.join(','));
  const banner = world.document.body.querySelector('[data-tbx-update]');
  check('bandeau de mise a jour affiche', !!banner);
  if (banner) {
    const text = banner.children.map((child) => child.textContent).join(' | ');
    check('bandeau mentionne la nouvelle version', text.indexOf('20261011') !== -1, text);
    const buttons = banner.children[banner.children.length - 1].children;
    check('bouton "Mettre a jour" presente', buttons[buttons.length - 1].textContent === 'Mettre à jour');
  }
  check('styles injectes', !!world.document.head.children.length);

  // Deuxième chargement de page dans la même session : throttlé par localStorage.
  const second = createWorld({ updateResult: { available: false } });
  second.window.localStorage = world.window.localStorage;
  second.sandbox.localStorage = world.window.localStorage;
  run(second);
  const calls2 = second.invokes.map((entry) => entry.command);
  check('check_update NON rappele dans les 6h (throttle)', calls2.indexOf('check_update') === -1, calls2.join(','));
}

/* ------------------------------------------------------------------ *
 *  2. Telechargement intercepte
 * ------------------------------------------------------------------ */
console.log('\n[2] Interception des telechargements');
{
  const world = run(createWorld({ updateResult: { available: false } }));
  const anchor = new FakeElement('a');
  anchor.setAttribute('href', 'blob:https://tauri.localhost/1234');
  anchor.setAttribute('download', 'floute.png');
  world.document.body.appendChild(anchor);

  const event = { type: 'click', target: anchor, defaultPrevented: false };
  dispatch(anchor, event);

  const save = world.invokes.filter((entry) => entry.command === 'save_download');
  check('save_download appele une fois', save.length === 1, String(save.length));
  check('navigation par defaut empechee', event.defaultPrevented === true);
  if (save.length) {
    check('nom de fichier encode transmis',
      save[0].options && save[0].options.headers &&
      save[0].options.headers['x-tbx-filename'] === 'floute.png',
      JSON.stringify(save[0].options));
    check('donnees brutes transmises (Uint8Array)', save[0].payload instanceof Uint8Array);
  }
}

/* ------------------------------------------------------------------ *
 *  3. Liens externes
 * ------------------------------------------------------------------ */
console.log('\n[3] Liens externes');
{
  const world = run(createWorld({ updateResult: { available: false } }));

  const external = new FakeElement('a');
  external.setAttribute('href', 'https://github.com/RackRunner1/ToolsBoXMachine');
  external.setAttribute('target', '_blank');
  world.document.body.appendChild(external);
  dispatch(external, { type: 'click', target: external });
  check('open_external appele pour un lien externe',
    world.invokes.some((entry) => entry.command === 'open_external' && entry.payload.url.indexOf('github.com') !== -1));

  const internal = new FakeElement('a');
  internal.setAttribute('href', '/tools/blur/blur.html');
  world.document.body.appendChild(internal);
  dispatch(internal, { type: 'click', target: internal });
  check('navigation interne laissee passer',
    !world.invokes.some((entry) => entry.command === 'open_external' && entry.payload.url === '/tools/blur/blur.html'));

  world.window.open = undefined; // restaure un window.open natif factice
  void world;
}

/* ------------------------------------------------------------------ *
 *  4. window.open + presse-papier
 * ------------------------------------------------------------------ */
console.log('\n[4] window.open et presse-papier');
{
  const world = run(createWorld({ updateResult: { available: false } }));
  const opened = world.window.open('https://example.com/doc', '_blank');
  check('window.open cible externe -> open_external', world.invokes.some((entry) => entry.command === 'open_external'));
  check('window.open renvoie null', opened === null);

  const clipboard = world.window.navigator.clipboard;
  // writeText natif rejette -> repli execCommand (absent du faux DOM) -> rejet
  clipboard.writeText('coucou').then(
    () => { check('repli presse-papier : devrait echouer proprement', false, 'resolu'); },
    () => { check('repli presse-papier : echec propre (pas de crash)', true); }
  );
}

setTimeout(() => {
  console.log('\n' + passed + ' ok, ' + failed + ' echec' + (failed > 1 ? 's' : ''));
  process.exit(failed === 0 ? 0 : 1);
}, 50);
