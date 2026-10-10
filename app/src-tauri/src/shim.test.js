/* Functional test of the ToolsBoXApp shim with a fake DOM (Node). */
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

/* Let pending promises and timers run. */
function tick(times) {
  let chain = Promise.resolve();
  for (let i = 0; i < (times || 5); i++) {
    chain = chain.then(() => new Promise((resolve) => setTimeout(resolve, 0)));
  }
  return chain;
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
    return this.querySelectorAll(selector)[0] || null;
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
  getElementById(id) {
    const walk = (node) => {
      for (const child of node.children) {
        if (child.id === id) { return child; }
        const found = walk(child);
        if (found) { return found; }
      }
      return null;
    };
    return walk(this);
  }
  closest(selector) {
    let node = this;
    while (node) {
      if (matches(node, selector)) { return node; }
      node = node.parentNode;
    }
    return null;
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
  if (/^\[data-tbx-toast\]$/.test(selector)) {
    return element.hasAttribute('data-tbx-toast');
  }
  return false;
}

function dispatch(target, event) {
  // Walk from the target up to the document (capture phase).
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
 *  Simulated world
 * ------------------------------------------------------------------ */
function createWorld() {
  const document = new FakeElement('#document');
  document.readyState = 'complete';
  document.head = new FakeElement('head');
  document.body = new FakeElement('body');
  document.head.parentNode = document;
  document.body.parentNode = document;
  document.createElement = (tag) => new FakeElement(tag);

  const invokes = [];
  const clipboard = {
    writeText: () => Promise.reject(new Error('navigator.clipboard unavailable in tests'))
  };

  const storageData = {};
  const storage = {
    getItem: (k) => (Object.prototype.hasOwnProperty.call(storageData, k) ? storageData[k] : null),
    setItem: (k, v) => { storageData[k] = String(v); },
    removeItem: (k) => { delete storageData[k]; }
  };

  const window = {
    document,
    navigator: { clipboard },
    localStorage: storage,
    console,
    setTimeout: (fn, ms) => setTimeout(fn, typeof ms === 'number' ? ms : 0),
    clearTimeout: (id) => clearTimeout(id),
    location: { pathname: '/index.html', href: 'https://tauri.localhost/index.html' }
  };
  window.window = window;
  window.self = window;

  window.__TAURI__ = {
    core: {
      invoke: (command, payload, options2) => {
        invokes.push({ command, payload, options: options2 });
        if (command === 'save_download') {
          return Promise.resolve('C:\\Users\\test\\Downloads\\file.png');
        }
        return Promise.resolve(undefined);
      }
    }
  };

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

  return { sandbox, document, invokes, window };
}

function run(world) {
  vm.runInContext(SHIM, world.sandbox, { filename: 'shim.js' });
  if (world.document.readyState === 'loading') {
    const handlers = world.document.listeners['DOMContentLoaded'] || [];
    for (const handler of handlers) { handler(); }
  }
  return world;
}

function isUint8Array(value) {
  return Object.prototype.toString.call(value) === '[object Uint8Array]';
}

/* ------------------------------------------------------------------ *
 *  Test suite (async: shim invokes resolve on later ticks)
 * ------------------------------------------------------------------ */
async function main() {
  /* -------------------------------------------------------------- *
   *  1. Intercepted download
   * -------------------------------------------------------------- */
  console.log('\n[1] Download interception');
  {
    const world = run(createWorld());
    await tick();
    check('styles injected', !!world.document.head.children.length);

    const anchor = new FakeElement('a');
    anchor.setAttribute('href', 'blob:https://tauri.localhost/1234');
    anchor.setAttribute('download', 'blurred.png');
    world.document.body.appendChild(anchor);

    const event = { type: 'click', target: anchor, defaultPrevented: false };
    dispatch(anchor, event);
    await tick(8);

    const saves = world.invokes.filter((entry) => entry.command === 'save_download');
    check('save_download called once', saves.length === 1, String(saves.length));
    check('default navigation prevented', event.defaultPrevented === true);
    if (saves.length) {
      check('encoded file name passed',
        saves[0].options && saves[0].options.headers &&
        saves[0].options.headers['x-tbx-filename'] === 'blurred.png',
        JSON.stringify(saves[0].options));
      check('raw bytes passed (Uint8Array)', isUint8Array(saves[0].payload));
    }
  }

  /* -------------------------------------------------------------- *
   *  2. External links
   * -------------------------------------------------------------- */
  console.log('\n[2] External links');
  {
    const world = run(createWorld());
    await tick();

    const external = new FakeElement('a');
    external.setAttribute('href', 'https://github.com/RackRunner1/ToolsBoXMachine');
    external.setAttribute('target', '_blank');
    world.document.body.appendChild(external);
    dispatch(external, { type: 'click', target: external });
    check('open_external called for an external link',
      world.invokes.some((entry) => entry.command === 'open_external' && entry.payload.url.indexOf('github.com') !== -1));

    const internal = new FakeElement('a');
    internal.setAttribute('href', '/tools/blur/blur.html');
    world.document.body.appendChild(internal);
    dispatch(internal, { type: 'click', target: internal });
    check('internal navigation left alone',
      !world.invokes.some((entry) => entry.command === 'open_external' && entry.payload.url === '/tools/blur/blur.html'));
  }

  /* -------------------------------------------------------------- *
   *  3. window.open and clipboard
   * -------------------------------------------------------------- */
  console.log('\n[3] window.open and clipboard');
  {
    const world = run(createWorld());
    await tick();

    const opened = world.window.open('https://example.com/doc', '_blank');
    check('window.open on external target -> open_external', world.invokes.some((entry) => entry.command === 'open_external'));
    check('window.open returns null', opened === null);

    const clipboard = world.window.navigator.clipboard;
    // The native writeText rejects -> execCommand fallback (missing here) -> clean rejection.
    try {
      await clipboard.writeText('hello');
      check('clipboard fallback: should reject cleanly', false, 'resolved');
    }
    catch (error) {
      check('clipboard fallback: clean failure (no crash)', true);
    }
  }

  console.log('\n' + passed + ' passed, ' + failed + ' failed');
  process.exit(failed === 0 ? 0 : 1);
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
