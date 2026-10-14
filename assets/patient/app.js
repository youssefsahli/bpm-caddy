// La page du patient : la messagerie de son officine, chiffrée de bout en
// bout avec WebCrypto. Le protocole est celui de src/patientchat.rs ; les
// valeurs de KAT y sont vérifiées par un test, et ici avant tout envoi.
'use strict';
// bpm-page: hors passerelle

const KAT = {
  client: 'e2ae25f709a44e263610be14252b440e71ddcfbcfdfbea76aecaf8be5688d788',
  server: 'c4441c7f787363f4e263bee9b3043830e6328e4f032671d2a3e27e199e330167',
  closed: '4620447ef9ec84d374c61162b26bf1ee13dff689608b6eed5ee639fa21cf006c',
  sealed: '7569bf2885d9a3b1d898da2a820ca83b350bbdbf2c7d9f',
};
const enc = new TextEncoder();
const dec = new TextDecoder();
const SALT = enc.encode('bpm-caddy-patient-1');
const MAGIC = enc.encode('BPMP1');
const CLOSE_ABSENT = 4004;
const STORE = 'bpm-caddy-messagerie';
const POLL_MS = 30000;
const TALK_MS = 30000;

const $ = (id) => document.getElementById(id);

function hex(bytes) {
  return Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('');
}

function cat(...parts) {
  const out = new Uint8Array(parts.reduce((n, p) => n + p.length, 0));
  let at = 0;
  for (const p of parts) {
    out.set(p, at);
    at += p.length;
  }
  return out;
}

function fromB64url(text) {
  const b = atob(text.replace(/-/g, '+').replace(/_/g, '/'));
  return Uint8Array.from(b, (c) => c.charCodeAt(0));
}

function random(n) {
  return crypto.getRandomValues(new Uint8Array(n));
}

async function hkdf(salt, ikm, info) {
  const k = await crypto.subtle.importKey('raw', ikm, 'HKDF', false, ['deriveBits']);
  const bits = await crypto.subtle.deriveBits(
    { name: 'HKDF', hash: 'SHA-256', salt, info: enc.encode(info) }, k, 256);
  return new Uint8Array(bits);
}

async function hmac(key, ...parts) {
  const k = await crypto.subtle.importKey(
    'raw', key, { name: 'HMAC', hash: 'SHA-256' }, false, ['sign']);
  return new Uint8Array(await crypto.subtle.sign('HMAC', k, cat(...parts)));
}

const authKey = (k) => hkdf(SALT, k, 'auth');

function nonce(n) {
  const iv = new Uint8Array(12);
  new DataView(iv.buffer).setBigUint64(4, BigInt(n));
  return iv;
}

async function sealer(raw) {
  const key = await crypto.subtle.importKey('raw', raw, 'AES-GCM', false, ['encrypt', 'decrypt']);
  let n = 0;
  return {
    seal: async (plain) => new Uint8Array(
      await crypto.subtle.encrypt({ name: 'AES-GCM', iv: nonce(n++) }, key, plain)),
    open: async (sealed) => new Uint8Array(
      await crypto.subtle.decrypt({ name: 'AES-GCM', iv: nonce(n++) }, key, sealed)),
  };
}

async function session(k, nc, ns) {
  const salt = cat(nc, ns);
  return {
    up: await sealer(await hkdf(salt, k, 'patient>officine')),
    down: await sealer(await hkdf(salt, k, 'officine>patient')),
  };
}

// The browser seals as src/patientchat.rs does, or it says nothing.
async function selfTest() {
  try {
    const k = new Uint8Array(32).fill(0x11);
    const nc = new Uint8Array(16).fill(0x22);
    const ns = new Uint8Array(16).fill(0x33);
    const a = await authKey(k);
    if (hex(await hmac(a, enc.encode('client'), nc)) !== KAT.client) return false;
    if (hex(await hmac(a, enc.encode('server'), nc, ns)) !== KAT.server) return false;
    if (hex(await hmac(a, enc.encode('ferme'), nc)) !== KAT.closed) return false;
    const s = await session(k, nc, ns);
    return hex(await s.up.seal(enc.encode('bonjour'))) === KAT.sealed;
  } catch (e) {
    return false;
  }
}

function sameBytes(a, b) {
  if (a.length !== b.length) return false;
  let d = 0;
  for (let i = 0; i < a.length; i++) d |= a[i] ^ b[i];
  return d === 0;
}

// One conversation: hello, the post's proof, the request, the answer.
function converse(state, request) {
  return new Promise((resolve, reject) => {
    // The page is published apart from the gateway: the link says which
    // one (`p`). Plain ws only for a gateway on this machine (tests).
    const scheme = /^127\.0\.0\.1:/.test(state.p) ? 'ws:' : 'wss:';
    const ws = new WebSocket(`${scheme}//${state.p}/ws/${state.b}`);
    ws.binaryType = 'arraybuffer';
    const inbox = [];
    let waiter = null;
    let finished = false;
    const finish = (fn, v) => {
      if (finished) return;
      finished = true;
      clearTimeout(timer);
      try { ws.close(); } catch (e) { /* already closed */ }
      fn(v);
    };
    const timer = setTimeout(() => finish(reject, new Error('délai')), TALK_MS);
    const next = () => new Promise((ok, ko) => {
      if (inbox.length) ok(inbox.shift());
      else waiter = { ok, ko };
    });
    ws.onmessage = (ev) => {
      const m = new Uint8Array(ev.data);
      if (waiter) { const w = waiter; waiter = null; w.ok(m); } else inbox.push(m);
    };
    ws.onclose = (ev) => {
      if (ev.code === CLOSE_ABSENT) finish(resolve, { absent: true });
      else if (waiter) { const w = waiter; waiter = null; w.ko(new Error('fermée')); }
      else finish(reject, new Error('fermée'));
    };
    ws.onerror = () => {};
    ws.onopen = async () => {
      try {
        const key = state.key;
        const nc = random(16);
        const a = await authKey(key);
        ws.send(cat(MAGIC, nc, await hmac(a, enc.encode('client'), nc)));
        const back = await next();
        // « Closed » counts only signed by whoever holds this key: a bare
        // word from the gateway would shut the patient out for good.
        if (back.length === 37 && dec.decode(back.slice(0, 5)) === 'FERME') {
          if (!sameBytes(await hmac(a, enc.encode('ferme'), nc), back.slice(5))) {
            throw new Error('preuve');
          }
          return finish(resolve, { closed: true });
        }
        if (dec.decode(back) === 'INCONNU') return finish(resolve, { unknown: true });
        if (back.length !== 48) throw new Error('réponse');
        const ns = back.slice(0, 16);
        if (!sameBytes(await hmac(a, enc.encode('server'), nc, ns), back.slice(16))) {
          throw new Error('preuve');
        }
        const s = await session(key, nc, ns);
        ws.send(await s.up.seal(enc.encode(JSON.stringify(request))));
        const answer = JSON.parse(dec.decode(await s.down.open(await next())));
        finish(resolve, { answer });
      } catch (e) {
        finish(reject, e);
      }
    };
  });
}

// --- What the phone keeps -------------------------------------------------

let state = null;
let storable = true;

// One conversation per officine (its box): every officine's page lives
// on the same site, and one patient may write to two of them.
const slot = (b) => `${STORE}:${b}`;

function load(b) {
  try {
    const raw = localStorage.getItem(slot(b));
    return raw ? JSON.parse(raw) : null;
  } catch (e) {
    storable = false;
    return null;
  }
}

// The officines this phone holds a conversation with.
function boxesKept() {
  try {
    return Object.keys(localStorage)
      .filter((k) => k.startsWith(`${STORE}:`))
      .map((k) => k.slice(STORE.length + 1));
  } catch (e) {
    return [];
  }
}

function save() {
  if (!state) return;
  try {
    const { key, ...kept } = state;
    localStorage.setItem(slot(state.b), JSON.stringify(kept));
  } catch (e) {
    storable = false;
  }
}

function fromLink() {
  const p = new URLSearchParams(location.hash.slice(1));
  const b = (p.get('b') || '').toLowerCase();
  const k = p.get('k') || '';
  // A code of 0.433.0 named no gateway: that gateway sends it here with
  // its own name in the query.
  const g = (p.get('p') || new URLSearchParams(location.search).get('p') || '').toLowerCase();
  if (!/^[0-9a-f]{32}$/.test(b)) return null;
  if (!/^[A-Za-z0-9_-]{43}$/.test(k)) return { b };
  if (!/^(\[[0-9a-f:]+\]|[a-z0-9.-]+)(:[0-9]{1,5})?$/.test(g)) return null;
  return { b, k, p: g };
}

function newId() {
  return hex(random(8));
}

function nowText() {
  const d = new Date();
  const z = (n) => String(n).padStart(2, '0');
  return `${z(d.getDate())}/${z(d.getMonth() + 1)}/${d.getFullYear()} ${z(d.getHours())}:${z(d.getMinutes())}`;
}

// --- The page ---------------------------------------------------------------

function say(text, error) {
  const e = $('etat');
  e.textContent = text || '';
  e.className = error ? 'erreur' : '';
}

function draw() {
  const fil = $('fil');
  fil.replaceChildren();
  if (state.officine) $('officine').textContent = state.officine;
  const who = state.who || {};
  $('rt').textContent = [state.officine || 'votre pharmacie', who.adresse, who.telephone]
    .filter(Boolean).join(', ');
  $('horaires').textContent = who.horaires ? `Ouverte : ${who.horaires}` : '';
  $('horaires').hidden = !who.horaires;
  if (!state.messages.length) {
    const p = document.createElement('p');
    p.className = 'vide';
    p.textContent = 'Aucun message pour l\'instant.';
    fil.append(p);
  }
  for (const m of state.messages) {
    const div = document.createElement('div');
    div.className = `bulle ${m.mine ? 'moi' : 'eux'}`;
    div.textContent = m.text;
    const when = document.createElement('span');
    when.className = 'quand';
    when.textContent = m.mine ? `${m.at}${m.pending ? ' · en attente' : ' · reçu'}` : m.at;
    div.append(when);
    fil.append(div);
  }
  const left = 2000 - $('texte').value.length;
  $('compte').textContent = left < 200 ? `${left} caractères restants` : '';
}

let busy = false;

async function sync(quiet) {
  if (busy || !state || state.over) return;
  busy = true;
  const pending = state.messages.filter((m) => m.mine && m.pending);
  if (!quiet) say(pending.length ? 'Envoi…' : 'Mise à jour…');
  try {
    const r = await converse(state, {
      out: pending.slice(0, 20).map((m) => ({ id: m.id, at: m.at, text: m.text })),
      // By number, not « since »: each post draws its own.
      seen: state.messages.filter((m) => !m.mine).map((m) => m.id).slice(-500),
    });
    if (r.absent) {
      say(pending.length
        ? 'La pharmacie n\'est pas joignable pour l\'instant. Votre message reste sur ce téléphone et partira dès qu\'elle le sera : gardez cette page ouverte, ou revenez plus tard.'
        : 'La pharmacie n\'est pas joignable pour l\'instant.');
    } else if (r.closed) {
      state.over = true;
      say('La pharmacie a fermé cet accès. Demandez-lui un nouveau code si besoin.', true);
      $('ecrire').hidden = true;
    } else if (r.unknown) {
      // A code just handed over may not have reached the post that
      // answered yet: tried again, never given up on.
      say('Ce code n\'est pas encore reconnu par la pharmacie. Nouvel essai dans un moment ; s\'il ne l\'est toujours pas demain, demandez-lui un nouveau code.', true);
    } else {
      const a = r.answer;
      const got = new Set(a.got || []);
      for (const m of state.messages) if (m.mine && got.has(m.id)) m.pending = false;
      for (const m of a.in || []) {
        if (!state.messages.some((x) => !x.mine && x.id === m.id)) {
          state.messages.push({ id: m.id, at: m.at, text: m.text, mine: false });
        }
      }
      if (a.officine) state.officine = a.officine;
      state.who = { adresse: a.adresse || '', telephone: a.telephone || '', horaires: a.horaires || '' };
      say(pending.length ? 'Message reçu par la pharmacie.' : '');
    }
  } catch (e) {
    if (!quiet || pending.length) say('La connexion a échoué. Nouvel essai dans un moment.', true);
  } finally {
    busy = false;
    save();
    draw();
  }
}

function send() {
  const t = $('texte');
  const text = t.value.trim();
  if (!text) return;
  state.messages.push({ id: newId(), at: nowText(), text: text.slice(0, 2000), mine: true, pending: true });
  t.value = '';
  save();
  draw();
  sync(false);
}

async function start() {
  if (!window.crypto || !crypto.subtle || !(await selfTest())) {
    say('Ce navigateur ne permet pas le chiffrement de la messagerie. Essayez avec un navigateur à jour.', true);
    return;
  }
  const link = fromLink();
  // Which officine: the link's, or the only one this phone knows.
  const boxes = boxesKept();
  const b = link ? link.b : (boxes.length === 1 ? boxes[0] : '');
  const kept = b ? load(b) : null;
  if (link && link.k) {
    state = kept && kept.k === link.k
      ? kept
      : { b: link.b, k: link.k, messages: [], officine: '' };
    state.p = link.p;
  } else if (kept && kept.b && kept.k && kept.p) {
    state = kept;
  } else {
    say('Ouvrez cette page en scannant le code remis par votre pharmacie.', true);
    return;
  }
  state.key = fromB64url(state.k);
  save();
  // The key out of the address bar once the phone keeps it: a screenshot
  // or a shared link does not carry it.
  // Only the box stays, which names the officine and not the patient: a
  // reload finds the conversation, a screenshot does not carry the key.
  if (link && storable) history.replaceState(null, '', `${location.pathname}#b=${state.b}`);
  $('ecrire').hidden = !!state.over;
  $('effacer').hidden = false;
  if (state.over) say('La pharmacie a fermé cet accès. Demandez-lui un nouveau code si besoin.', true);
  $('texte').addEventListener('input', draw);
  $('envoyer').addEventListener('click', send);
  $('effacer').addEventListener('click', () => {
    if (!confirm('Effacer cette conversation de ce téléphone ? Il faudra scanner de nouveau le code pour écrire à la pharmacie.')) return;
    try { localStorage.removeItem(slot(state.b)); } catch (e) { /* nothing kept */ }
    location.replace(location.pathname);
  });
  document.addEventListener('visibilitychange', () => {
    if (document.visibilityState === 'visible') sync(true);
  });
  setInterval(() => { if (document.visibilityState === 'visible') sync(true); }, POLL_MS);
  draw();
  sync(true);
}

start();
